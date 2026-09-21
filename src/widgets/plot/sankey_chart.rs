//! Sankey diagrams: flow between nodes across columns.
//!
//! Ported from `gpui-kit`'s `plot/shape/sankey` (Apache-2.0), rebuilt on iced's
//! canvas.
//!
//! # The layout
//!
//! The algorithm is d3-sankey's, in four stages:
//!
//! 1. **Layering** — each node is placed in a column, one past its deepest
//!    predecessor. Cycles are broken first, or the walk would not terminate.
//! 2. **Breadths** — each node's height is its throughput, scaled so the
//!    busiest column fits the available height.
//! 3. **Relaxation** — nodes are nudged toward the weighted centre of their
//!    neighbours, a fixed number of passes with a cooling factor. This is what
//!    straightens the flows; without it the diagram is a tangle.
//! 4. **Centring** — each column is centred in the extent, so a sparse column
//!    does not sit at the top with empty space below.
//!
//! The result is cached per frame rather than per node, because the relaxation
//! is iterative and would be far too slow to run during drawing.

use crate::theme::Theme;
use crate::widgets::display::Tone;
use crate::widgets::plot::series_color;
use crate::widgets::plot::tooltip::{draw_tooltip, TooltipContent};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Element, Font, Length, Pixels, Point, Rectangle};

/// How the columns are aligned horizontally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SankeyAlign {
    /// Spread the columns to fill the width evenly.
    #[default]
    Justify,
    /// Pack the columns against the left edge.
    Left,
    /// Pack the columns against the right edge.
    Right,
    /// Centre the columns.
    Center,
}

/// How node throughput is converted to height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SankeyValueScale {
    /// Height proportional to the value.
    #[default]
    Linear,
    /// Height proportional to the square root, which compresses a wide range so
    /// that a small flow next to a huge one stays visible.
    Sqrt,
}

/// One node of a Sankey diagram.
#[derive(Debug, Clone)]
#[must_use = "a SankeyNode does nothing unless it is given to a SankeyChart"]
pub struct SankeyNode {
    label: String,
    tone: Option<Tone>,
    color: Option<Color>,
}

impl SankeyNode {
    /// Creates a labelled node.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            tone: None,
            color: None,
        }
    }

    /// Sets the node's color tone.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Sets an explicit color, overriding the tone.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The node's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// A flow from one node to another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SankeyLink {
    /// The index of the source node.
    pub source: usize,
    /// The index of the target node.
    pub target: usize,
    /// The flow's magnitude.
    pub value: f64,
}

impl SankeyLink {
    /// Creates a link between two nodes.
    pub fn new(source: usize, target: usize, value: f64) -> Self {
        Self {
            source,
            target,
            value,
        }
    }
}

/// A node's resolved geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
struct NodeBox {
    /// The node's column.
    layer: usize,
    /// The left edge.
    x: f32,
    /// The top edge.
    y: f32,
    /// The height, which is its throughput.
    height: f32,
    /// How much of the node's height has been claimed by outgoing links so far.
    source_offset: f32,
    /// How much has been claimed by incoming links so far.
    target_offset: f32,
}

/// A link's resolved geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LinkBand {
    source: usize,
    target: usize,
    value: f64,
    /// The top edge at the source end.
    source_y: f32,
    /// The top edge at the target end.
    target_y: f32,
    /// The band's thickness.
    thickness: f32,
}

/// A Sankey diagram.
#[must_use = "a SankeyChart does nothing unless it is turned into an Element"]
pub struct SankeyChart {
    nodes: Vec<SankeyNode>,
    links: Vec<SankeyLink>,
    align: SankeyAlign,
    value_scale: SankeyValueScale,
    node_width: f32,
    node_padding: f32,
    link_opacity: f32,
    iterations: usize,
    show_labels: bool,
    show_tooltip: bool,
    height: f32,
    tone: Tone,
}

impl SankeyChart {
    /// Creates a diagram from its nodes and links.
    ///
    /// A link naming a node index that does not exist is ignored rather than
    /// panicking; a caller building links from data should not be able to crash
    /// the render.
    pub fn new(nodes: Vec<SankeyNode>, links: Vec<SankeyLink>) -> Self {
        Self {
            nodes,
            links,
            align: SankeyAlign::default(),
            value_scale: SankeyValueScale::default(),
            node_width: 16.0,
            node_padding: 10.0,
            link_opacity: 0.35,
            // Six passes is d3-sankey's default: enough to straighten the
            // flows, few enough to stay imperceptible per frame.
            iterations: 6,
            show_labels: true,
            show_tooltip: true,
            height: 300.0,
            tone: Tone::Primary,
        }
    }

    /// Sets how the columns are aligned.
    pub fn align(mut self, align: SankeyAlign) -> Self {
        self.align = align;
        self
    }

    /// Sets how throughput maps to height.
    pub fn value_scale(mut self, scale: SankeyValueScale) -> Self {
        self.value_scale = scale;
        self
    }

    /// Sets the width of each node box.
    pub fn node_width(mut self, width: f32) -> Self {
        self.node_width = width.clamp(1.0, 80.0);
        self
    }

    /// Sets the vertical gap between nodes in the same column.
    pub fn node_padding(mut self, padding: f32) -> Self {
        self.node_padding = padding.max(0.0);
        self
    }

    /// Sets the opacity of the flow ribbons, from 0 to 1.
    pub fn link_opacity(mut self, opacity: f32) -> Self {
        self.link_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets how many relaxation passes to run.
    pub fn iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations.clamp(0, 32);
        self
    }

    /// Turns the node labels on or off.
    pub fn labels(mut self, visible: bool) -> Self {
        self.show_labels = visible;
        self
    }

    /// Turns the hover tooltip on or off.
    pub fn tooltip(mut self, tooltip: bool) -> Self {
        self.show_tooltip = tooltip;
        self
    }

    /// Sets the chart's height in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(1.0);
        self
    }

    /// Sets the default color tone for nodes without an explicit color.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Moves the builder into the canvas program that draws it.
    fn into_program(self) -> SankeyProgram {
        SankeyProgram {
            nodes: self.nodes,
            links: self.links,
            align: self.align,
            value_scale: self.value_scale,
            node_width: self.node_width,
            node_padding: self.node_padding,
            link_opacity: self.link_opacity,
            iterations: self.iterations,
            show_labels: self.show_labels,
            show_tooltip: self.show_tooltip,
            tone: self.tone,
        }
    }

    /// Converts the chart into an [`Element`].
    #[must_use]
    pub fn into_element<Message: 'static>(self) -> Element<'static, Message, Theme> {
        let height = self.height;

        Canvas::new(self.into_program())
            .width(Length::Fill)
            .height(Length::Fixed(height))
            .into()
    }
}

impl<Message: 'static> From<SankeyChart> for Element<'static, Message, Theme> {
    fn from(chart: SankeyChart) -> Self {
        chart.into_element()
    }
}

/// A laid-out diagram, ready to draw.
#[derive(Debug, Clone, Default)]
struct Layout {
    nodes: Vec<NodeBox>,
    links: Vec<LinkBand>,
    layer_count: usize,
}

impl Layout {
    /// Whether there is anything to draw.
    fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// The canvas program that draws a Sankey diagram.
///
/// Mirrors the builder's display toggles.
#[allow(clippy::struct_excessive_bools)]
struct SankeyProgram {
    nodes: Vec<SankeyNode>,
    links: Vec<SankeyLink>,
    align: SankeyAlign,
    value_scale: SankeyValueScale,
    node_width: f32,
    node_padding: f32,
    link_opacity: f32,
    iterations: usize,
    show_labels: bool,
    show_tooltip: bool,
    tone: Tone,
}

impl SankeyProgram {
    /// The color a node is drawn in.
    ///
    /// An explicit color wins, then the node's own tone, then the shared palette.
    /// The builder's tone is the last resort rather than the first, so one
    /// `tone(..)` call does not repaint every node the caller did not color.
    fn color_of(&self, index: usize, theme: &Theme) -> Color {
        let node = &self.nodes[index];

        node.color
            .or_else(|| node.tone.map(|tone| tone.accent(theme)))
            .unwrap_or_else(|| {
                if self.nodes.len() == 1 {
                    self.tone.accent(theme)
                } else {
                    series_color(theme, index)
                }
            })
    }

    /// The links that survive validation: indices in range, positive values.
    fn valid_links(&self) -> Vec<SankeyLink> {
        self.links
            .iter()
            .filter(|link| {
                link.source < self.nodes.len()
                    && link.target < self.nodes.len()
                    && link.source != link.target
                    && link.value.is_finite()
                    && link.value > 0.0
            })
            .copied()
            .collect()
    }

    /// Assigns each node a column, breaking cycles first.
    ///
    /// A cycle (A feeds B feeds A) would make the depth walk non-terminating, so
    /// the back edges are dropped before layering.
    fn layers(&self, links: &[SankeyLink]) -> (Vec<usize>, usize) {
        let count = self.nodes.len();
        let mut adjacency = vec![Vec::new(); count];

        for link in links {
            adjacency[link.source].push(link.target);
        }

        // An edge is dropped when its target is already on the current path,
        // which is what makes the graph acyclic.
        let mut state = vec![0_u8; count]; // 0 unvisited, 1 on path, 2 done
        let mut acyclic: Vec<Vec<usize>> = vec![Vec::new(); count];

        for node in 0..count {
            if state[node] == 0 {
                visit_acyclic(node, &adjacency, &mut state, &mut acyclic);
            }
        }

        // Depth is one past the deepest predecessor, computed in topological
        // order, which the acyclic walk above already provides.
        let mut layer = vec![0_usize; count];

        for node in 0..count {
            let has_predecessor = acyclic.iter().any(|targets| targets.contains(&node));

            if !has_predecessor {
                assign_layers(node, &acyclic, &mut layer, 0);
            }
        }

        let layer_count = layer.iter().copied().max().unwrap_or(0) + 1;

        (layer, layer_count)
    }

    /// Converts a value to a height, per the chosen scale.
    fn scaled(&self, value: f64) -> f64 {
        match self.value_scale {
            SankeyValueScale::Linear => value,
            SankeyValueScale::Sqrt => value.max(0.0).sqrt(),
        }
    }

    /// Runs the full layout for the given plot area.
    fn layout(&self, area: Rectangle) -> Layout {
        let links = self.valid_links();

        if self.nodes.is_empty() || area.width <= 1.0 || area.height <= 1.0 {
            return Layout::default();
        }

        let (layers, layer_count) = self.layers(&links);
        let count = self.nodes.len();

        // Throughput per node: the larger of what flows in and what flows out,
        // so a pure source or pure sink is sized by the side it actually has.
        let mut inflow = vec![0.0_f64; count];
        let mut outflow = vec![0.0_f64; count];

        for link in &links {
            outflow[link.source] += self.scaled(link.value);
            inflow[link.target] += self.scaled(link.value);
        }

        let throughput: Vec<f64> = (0..count)
            .map(|index| inflow[index].max(outflow[index]).max(0.0))
            .collect();

        // Scale so the busiest column fits, leaving room for the padding.
        let mut column_totals = vec![0.0_f64; layer_count];

        for index in 0..count {
            column_totals[layers[index]] += throughput[index];
        }

        let busiest = column_totals.iter().copied().fold(0.0_f64, f64::max);

        if busiest <= 0.0 {
            return Layout::default();
        }

        let columns_with_nodes = column_totals.iter().filter(|total| **total > 0.0).count();
        let usable_height = f64::from(
            (area.height - self.node_padding * (columns_with_nodes.max(1) as f32 - 1.0)).max(1.0),
        );

        let scale = usable_height / busiest;

        // Column positions depend on the alignment.
        let column_x = |layer: usize| -> f32 {
            let last = layer_count.saturating_sub(1).max(1);
            let span = (area.width - self.node_width).max(0.0);

            match self.align {
                SankeyAlign::Justify => area.x + span * layer as f32 / last as f32,
                SankeyAlign::Left => area.x + layer as f32 * (self.node_width + 8.0),
                SankeyAlign::Right => {
                    area.x + area.width
                        - self.node_width
                        - (last - layer) as f32 * (self.node_width + 8.0)
                }
                SankeyAlign::Center => {
                    let total = layer_count as f32 * self.node_width
                        + (layer_count.saturating_sub(1)) as f32 * 8.0;

                    area.x + (area.width - total) / 2.0 + layer as f32 * (self.node_width + 8.0)
                }
            }
        };

        // Stack each column's nodes.
        let mut boxes = vec![
            NodeBox {
                layer: 0,
                x: 0.0,
                y: 0.0,
                height: 0.0,
                source_offset: 0.0,
                target_offset: 0.0,
            };
            count
        ];

        for layer in 0..layer_count {
            let members: Vec<usize> = (0..count).filter(|i| layers[*i] == layer).collect();

            if members.is_empty() {
                continue;
            }

            let column_height: f32 = members
                .iter()
                .map(|index| (throughput[*index] * scale) as f32)
                .sum::<f32>()
                + self.node_padding * (members.len() - 1) as f32;

            let mut y = area.y + ((area.height - column_height) / 2.0).max(0.0);

            for index in members {
                let height = (throughput[index] * scale).max(1.0) as f32;

                boxes[index] = NodeBox {
                    layer,
                    x: column_x(layer),
                    y,
                    height,
                    source_offset: 0.0,
                    target_offset: 0.0,
                };

                y += height + self.node_padding;
            }
        }

        // Relaxation: pull each node toward the weighted centre of its
        // neighbours. Without this the flows cross needlessly.
        if self.iterations > 0 && layer_count > 2 {
            let mut order: Vec<Vec<usize>> = vec![Vec::new(); layer_count];

            for index in 0..count {
                order[layers[index]].push(index);
            }

            for pass in 0..self.iterations {
                // A cooling factor, as d3-sankey uses: later passes move less,
                // so the layout settles rather than oscillating.
                let alpha = (1.0 - pass as f32 / self.iterations as f32).max(0.05);

                // Alternate direction each pass so both sides of a node settle.
                let forward = pass % 2 == 0;

                let layer_range: Vec<usize> = if forward {
                    (1..layer_count).collect()
                } else {
                    (0..layer_count.saturating_sub(1)).rev().collect()
                };

                for layer in layer_range {
                    let members = &order[layer];

                    if members.len() < 2 {
                        continue;
                    }

                    // Desired centre for each node, weighted by link value.
                    for &node in members {
                        let mut weighted = 0.0_f32;
                        let mut total = 0.0_f32;

                        for link in &links {
                            let (other, is_source) = if forward {
                                if link.target != node {
                                    continue;
                                }
                                (link.source, false)
                            } else {
                                if link.source != node {
                                    continue;
                                }
                                (link.target, true)
                            };

                            let other_centre = boxes[other].y + boxes[other].height / 2.0;
                            let weight = link.value as f32;

                            let _ = is_source;

                            weighted += other_centre * weight;
                            total += weight;
                        }

                        if total > 0.0 {
                            let desired = weighted / total;
                            let current = boxes[node].y + boxes[node].height / 2.0;
                            boxes[node].y += (desired - current) * alpha;
                        }
                    }

                    // Relaxation moves nodes independently, which can leave them
                    // overlapping. The column is re-stacked in its current
                    // vertical order, keeping the ordering the relax produced
                    // while restoring the gaps.
                    let mut ordered = members.clone();
                    ordered.sort_by(|a, b| {
                        boxes[*a]
                            .y
                            .partial_cmp(&boxes[*b].y)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                    let column_height: f32 = ordered
                        .iter()
                        .map(|index| boxes[*index].height)
                        .sum::<f32>()
                        + self.node_padding * (ordered.len() - 1) as f32;

                    // The stack is centred in the area, which also keeps it
                    // inside the frame after the relax moved it.
                    let mut y = area.y + ((area.height - column_height) / 2.0).max(0.0);

                    for &node in &ordered {
                        boxes[node].y = y;
                        y += boxes[node].height + self.node_padding;
                    }
                }
            }
        }

        // Clamp every node inside the plot area, since relaxation can push one
        // past an edge.
        for node_box in &mut boxes {
            node_box.y = node_box
                .y
                .clamp(area.y, (area.y + area.height - node_box.height).max(area.y));
        }

        // Resolve the links against the final node positions, assigning each a
        // slice of its endpoints' heights.
        let mut bands = Vec::with_capacity(links.len());

        for link in &links {
            let thickness = (self.scaled(link.value) * scale).max(1.0) as f32;

            let source_y = boxes[link.source].y + boxes[link.source].source_offset;
            let target_y = boxes[link.target].y + boxes[link.target].target_offset;

            boxes[link.source].source_offset += thickness;
            boxes[link.target].target_offset += thickness;

            bands.push(LinkBand {
                source: link.source,
                target: link.target,
                value: link.value,
                source_y,
                target_y,
                thickness,
            });
        }

        Layout {
            nodes: boxes,
            links: bands,
            layer_count,
        }
    }

    /// Builds the tooltip for a hovered node.
    fn tooltip_content(&self, index: usize, theme: &Theme) -> TooltipContent {
        let links = self.valid_links();

        let inflow: f64 = links
            .iter()
            .filter(|link| link.target == index)
            .map(|link| link.value)
            .sum();
        let outflow: f64 = links
            .iter()
            .filter(|link| link.source == index)
            .map(|link| link.value)
            .sum();

        let mut content = TooltipContent::new(self.nodes[index].label.clone());

        if inflow > 0.0 {
            content = content.row("In", format_value(inflow), theme.colors().muted_foreground);
        }

        if outflow > 0.0 {
            content = content.row(
                "Out",
                format_value(outflow),
                theme.colors().muted_foreground,
            );
        }

        content
    }
}

impl<Message, Renderer> canvas::Program<Message, Theme, Renderer> for SankeyProgram
where
    Renderer:
        iced::advanced::graphics::geometry::Renderer + iced::advanced::text::Renderer<Font = Font>,
{
    /// The laid-out diagram, plus which node is hovered.
    ///
    /// The layout is cached here because the relaxation is iterative; running it
    /// per frame would be wasteful, and running it during `draw` while also
    /// hit-testing would mean doing it twice.
    type State = SankeyState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let area = self.plot_area(bounds);

        // The relaxation is iterative, so the layout is cached and only rebuilt
        // when the canvas is resized.
        let needs_layout = state.area != Some(area);
        if needs_layout {
            state.layout = self.layout(area);
            state.area = Some(area);
        }

        if let iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) = event {
            let target = self.hit_test(&state.layout, bounds, cursor);

            if state.hovered != target {
                state.hovered = target;
                return Some(canvas::Action::request_redraw());
            }
        }

        if needs_layout {
            return Some(canvas::Action::request_redraw());
        }

        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());
        let kit_theme: &crate::theme::Theme = theme;
        let colors = kit_theme.colors();

        let layout = &state.layout;

        if layout.is_empty() {
            return vec![frame.into_geometry()];
        }

        let _ = self.plot_area(bounds);

        let hovered = state.hovered;

        // The ribbons first, so the nodes sit on top of their own edges.
        for band in &layout.links {
            let (Some(source), Some(target)) =
                (layout.nodes.get(band.source), layout.nodes.get(band.target))
            else {
                continue;
            };

            let x0 = source.x + self.node_width;
            let x1 = target.x;

            // A cubic with horizontal control points gives the S-curve a flow
            // ribbon is recognised by.
            let control = (x1 - x0) * 0.5;

            let top = Path::new(|builder| {
                builder.move_to(Point::new(x0, band.source_y));
                builder.bezier_curve_to(
                    Point::new(x0 + control, band.source_y),
                    Point::new(x1 - control, band.target_y),
                    Point::new(x1, band.target_y),
                );
            });

            let bottom = Path::new(|builder| {
                builder.move_to(Point::new(x0, band.source_y + band.thickness));
                builder.bezier_curve_to(
                    Point::new(x0 + control, band.source_y + band.thickness),
                    Point::new(x1 - control, band.target_y + band.thickness),
                    Point::new(x1, band.target_y + band.thickness),
                );
            });

            let color = self.color_of(band.source, kit_theme);
            let dimmed = hovered.is_some_and(|index| index != band.source && index != band.target);

            let fill = Color {
                a: self.link_opacity * if dimmed { 0.3 } else { 1.0 },
                ..color
            };

            // The band is filled as the closed region between its two edges.
            let ribbon = Path::new(|builder| {
                builder.move_to(Point::new(x0, band.source_y));
                builder.bezier_curve_to(
                    Point::new(x0 + control, band.source_y),
                    Point::new(x1 - control, band.target_y),
                    Point::new(x1, band.target_y),
                );
                builder.line_to(Point::new(x1, band.target_y + band.thickness));
                builder.bezier_curve_to(
                    Point::new(x1 - control, band.target_y + band.thickness),
                    Point::new(x0 + control, band.source_y + band.thickness),
                    Point::new(x0, band.source_y + band.thickness),
                );
                builder.close();
            });

            let _ = (top, bottom);

            frame.fill(&ribbon, fill);
        }

        // The node boxes.
        for (index, node_box) in layout.nodes.iter().enumerate() {
            let mut color = self.color_of(index, kit_theme);

            if hovered.is_some_and(|hovered| hovered != index) {
                color = Color { a: 0.45, ..color };
            }

            frame.fill_rectangle(
                Point::new(node_box.x, node_box.y),
                iced::Size::new(self.node_width, node_box.height),
                color,
            );

            if hovered == Some(index) {
                frame.stroke(
                    &Path::rectangle(
                        Point::new(node_box.x, node_box.y),
                        iced::Size::new(self.node_width, node_box.height),
                    ),
                    Stroke::default()
                        .with_color(colors.foreground)
                        .with_width(1.5),
                );
            }
        }

        // Labels beside each node, on the outer side so they do not sit on the
        // ribbons.
        if self.show_labels {
            for (index, node_box) in layout.nodes.iter().enumerate() {
                let is_last_column = node_box.layer + 1 == layout.layer_count;
                let on_left = is_last_column && layout.layer_count > 1;

                let x = if on_left {
                    node_box.x - 6.0
                } else {
                    node_box.x + self.node_width + 6.0
                };

                let align = if on_left {
                    iced::alignment::Horizontal::Right
                } else {
                    iced::alignment::Horizontal::Left
                };

                let alpha = if hovered.is_some_and(|h| h != index) {
                    0.5
                } else {
                    1.0
                };

                frame.fill_text(canvas::Text {
                    content: self.nodes[index].label.clone(),
                    position: Point::new(x, node_box.y + node_box.height / 2.0),
                    color: Color {
                        a: alpha,
                        ..colors.foreground
                    },
                    size: Pixels(11.0),
                    align_x: align.into(),
                    align_y: iced::alignment::Vertical::Center,
                    font: Font::DEFAULT,
                    ..canvas::Text::default()
                });
            }
        }

        // The tooltip.
        if let (Some(index), true) = (hovered, self.show_tooltip) {
            let content = self.tooltip_content(index, kit_theme);
            let node_box = layout.nodes[index];

            draw_tooltip(
                &mut frame,
                &content,
                Point::new(
                    node_box.x + self.node_width,
                    node_box.y + node_box.height / 2.0,
                ),
                Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: bounds.width,
                    height: bounds.height,
                },
                kit_theme,
            );
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if self.show_tooltip && cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

impl SankeyProgram {
    /// The plot area, inset to leave room for the outer labels.
    fn plot_area(&self, bounds: Rectangle) -> Rectangle {
        // Labels sit outside the outermost columns, so both edges need a margin.
        let margin = if self.show_labels { 70.0 } else { 8.0 };

        Rectangle {
            x: margin,
            y: 8.0,
            width: (bounds.width - margin * 2.0).max(1.0),
            height: (bounds.height - 16.0).max(1.0),
        }
    }

    /// Which node the cursor is over, if any.
    ///
    /// A node box is the target, not a ribbon: a ribbon's endpoints are the
    /// meaningful thing and its middle is thin.
    fn hit_test(&self, layout: &Layout, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        if !self.show_tooltip || layout.is_empty() {
            return None;
        }

        let position = cursor.position_over(bounds)?;

        layout.nodes.iter().position(|node_box| {
            let rect = Rectangle {
                x: node_box.x,
                y: node_box.y,
                width: self.node_width,
                height: node_box.height,
            };

            rect.contains(position)
        })
    }
}

/// The cached layout of a Sankey program.
#[derive(Default)]
struct SankeyState {
    layout: Layout,
    hovered: Option<usize>,
    /// The area the cached layout was computed for, so it is rebuilt when the
    /// canvas is resized.
    area: Option<Rectangle>,
}

/// Walks the graph from `node`, recording the forward edges that survive.
///
/// An edge onto a node already on the current path is a back edge; dropping it
/// is what makes the graph acyclic, without which the depth walk would not
/// terminate.
fn visit_acyclic(
    node: usize,
    adjacency: &[Vec<usize>],
    state: &mut [u8],
    acyclic: &mut [Vec<usize>],
) {
    state[node] = 1;

    for &next in &adjacency[node] {
        match state[next] {
            0 => {
                acyclic[node].push(next);
                visit_acyclic(next, adjacency, state, acyclic);
            }
            // 1 means "on the current path": a back edge, dropped.
            1 => {}
            _ => acyclic[node].push(next),
        }
    }

    state[node] = 2;
}

/// Assigns `node` a layer one past its deepest predecessor.
fn assign_layers(node: usize, acyclic: &[Vec<usize>], layer: &mut [usize], depth: usize) {
    layer[node] = layer[node].max(depth);

    for &next in &acyclic[node] {
        assign_layers(next, acyclic, layer, depth + 1);
    }
}

/// Formats a flow value without trailing noise.
fn format_value(value: f64) -> String {
    if value.abs() >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("{:.1}k", value / 1_000.0)
    } else if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// The default vertical gap between nodes in a column.
#[must_use]
pub fn default_node_padding() -> f32 {
    10.0
}

#[cfg(test)]
mod tests {
    use super::{SankeyAlign, SankeyChart, SankeyLink, SankeyNode, SankeyValueScale};
    use crate::theme::Theme;
    use crate::widgets::display::Tone;
    use iced::Color;
    use iced::Rectangle;

    /// Only the type matters; no handler is ever invoked in these tests.
    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    fn nodes() -> Vec<SankeyNode> {
        vec![
            SankeyNode::new("Coal"),
            SankeyNode::new("Gas"),
            SankeyNode::new("Electricity"),
            SankeyNode::new("Industry"),
            SankeyNode::new("Homes"),
        ]
    }

    fn links() -> Vec<SankeyLink> {
        vec![
            SankeyLink::new(0, 2, 40.0),
            SankeyLink::new(1, 2, 25.0),
            SankeyLink::new(2, 3, 45.0),
            SankeyLink::new(2, 4, 20.0),
        ]
    }

    fn program_of(chart: SankeyChart) -> super::SankeyProgram {
        chart.into_program()
    }

    fn area() -> Rectangle {
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        }
    }

    #[test]
    fn a_sankey_chart_renders() {
        let element: iced::Element<'_, Message, Theme> =
            SankeyChart::new(nodes(), links()).into_element();

        drop(element);
    }

    #[test]
    fn the_layers_follow_the_flow() {
        let program = program_of(SankeyChart::new(nodes(), links()));
        let valid = program.valid_links();
        let (layers, count) = program.layers(&valid);

        // Sources first, then the middle node, then the sinks.
        assert_eq!(layers[0], 0);
        assert_eq!(layers[1], 0);
        assert_eq!(layers[2], 1, "the middle node follows its sources");
        assert_eq!(layers[3], 2);
        assert_eq!(layers[4], 2);
        assert_eq!(count, 3);
    }

    #[test]
    fn a_cycle_is_broken_rather_than_hanging() {
        // Without cycle breaking the depth walk would not terminate.
        let cyclic = vec![
            SankeyLink::new(0, 1, 1.0),
            SankeyLink::new(1, 2, 1.0),
            SankeyLink::new(2, 0, 1.0),
        ];

        let program = program_of(SankeyChart::new(nodes(), cyclic));
        let valid = program.valid_links();
        let (_layers, count) = program.layers(&valid);

        assert!(count >= 1, "a cycle still produces a layout");
    }

    #[test]
    fn a_self_link_is_dropped() {
        // A node feeding itself has no meaning in a flow diagram.
        let program = program_of(SankeyChart::new(
            nodes(),
            vec![SankeyLink::new(0, 0, 10.0), SankeyLink::new(0, 1, 5.0)],
        ));

        let valid = program.valid_links();
        assert_eq!(valid.len(), 1);
    }

    #[test]
    fn links_naming_missing_nodes_are_dropped() {
        // A caller building links from data must not be able to crash the render.
        let program = program_of(SankeyChart::new(
            nodes(),
            vec![
                SankeyLink::new(0, 99, 10.0),
                SankeyLink::new(99, 1, 10.0),
                SankeyLink::new(0, 1, 5.0),
            ],
        ));

        let valid = program.valid_links();
        assert_eq!(valid.len(), 1);
    }

    #[test]
    fn non_positive_and_non_finite_links_are_dropped() {
        let program = program_of(SankeyChart::new(
            nodes(),
            vec![
                SankeyLink::new(0, 1, 0.0),
                SankeyLink::new(0, 1, -5.0),
                SankeyLink::new(0, 1, f64::NAN),
                SankeyLink::new(0, 2, 5.0),
            ],
        ));

        let valid = program.valid_links();
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].target, 2);
    }

    #[test]
    fn the_layout_places_each_node_in_its_layer() {
        let program = program_of(SankeyChart::new(nodes(), links()));
        let layout = program.layout(area());

        assert_eq!(layout.nodes.len(), 5);
        assert_eq!(layout.layer_count, 3);

        assert_eq!(layout.nodes[0].layer, 0);
        assert_eq!(layout.nodes[2].layer, 1);
        assert_eq!(layout.nodes[3].layer, 2);

        // Later columns sit further right.
        assert!(layout.nodes[0].x < layout.nodes[2].x);
        assert!(layout.nodes[2].x < layout.nodes[3].x);
    }

    #[test]
    fn a_nodes_height_follows_its_throughput() {
        let program = program_of(SankeyChart::new(nodes(), links()));
        let layout = program.layout(area());

        // "Electricity" carries 65 (40 + 25) while "Coal" carries 40.
        assert!(
            layout.nodes[2].height > layout.nodes[0].height,
            "the busier node must be taller"
        );
    }

    #[test]
    fn every_node_stays_inside_the_plot_area() {
        // Relaxation can push a node past an edge, so the clamp is what keeps
        // the diagram inside its frame.
        let program = program_of(SankeyChart::new(nodes(), links()));
        let layout = program.layout(area());

        for (index, node_box) in layout.nodes.iter().enumerate() {
            assert!(
                node_box.y >= area().y - 0.001,
                "node {index} starts above the area"
            );
            assert!(
                node_box.y + node_box.height <= area().y + area().height + 0.001,
                "node {index} ends below the area"
            );
        }
    }

    #[test]
    fn nodes_in_a_column_do_not_overlap() {
        let program = program_of(SankeyChart::new(nodes(), links()));
        let layout = program.layout(area());

        for a in 0..layout.nodes.len() {
            for b in 0..layout.nodes.len() {
                if a == b || layout.nodes[a].layer != layout.nodes[b].layer {
                    continue;
                }

                let (first, second) = (layout.nodes[a], layout.nodes[b]);
                let (upper, lower) = if first.y <= second.y {
                    (first, second)
                } else {
                    (second, first)
                };

                assert!(
                    upper.y + upper.height <= lower.y + 0.001,
                    "nodes overlap in column {}",
                    upper.layer
                );
            }
        }
    }

    #[test]
    fn the_scale_can_compress_a_wide_range() {
        let wide = vec![
            SankeyNode::new("Huge"),
            SankeyNode::new("Tiny"),
            SankeyNode::new("Sink"),
        ];
        let links = vec![SankeyLink::new(0, 2, 10_000.0), SankeyLink::new(1, 2, 1.0)];

        let linear = program_of(SankeyChart::new(wide.clone(), links.clone()));
        let sqrt = program_of(SankeyChart::new(wide, links).value_scale(SankeyValueScale::Sqrt));

        let linear_layout = linear.layout(area());
        let sqrt_layout = sqrt.layout(area());

        let linear_ratio = linear_layout.nodes[0].height / linear_layout.nodes[1].height.max(1.0);
        let sqrt_ratio = sqrt_layout.nodes[0].height / sqrt_layout.nodes[1].height.max(1.0);

        assert!(
            sqrt_ratio < linear_ratio,
            "the square-root scale must compress the ratio: {sqrt_ratio} vs {linear_ratio}"
        );
    }

    #[test]
    fn every_alignment_renders_and_places_columns() {
        for align in [
            SankeyAlign::Justify,
            SankeyAlign::Left,
            SankeyAlign::Right,
            SankeyAlign::Center,
        ] {
            let program = program_of(SankeyChart::new(nodes(), links()).align(align));
            let layout = program.layout(area());

            assert_eq!(layout.nodes.len(), 5, "{align:?} must still lay out");

            // Every alignment keeps the columns in order.
            assert!(layout.nodes[0].x < layout.nodes[2].x, "{align:?}");
        }
    }

    #[test]
    fn links_know_their_endpoints() {
        let program = program_of(SankeyChart::new(nodes(), links()));
        let layout = program.layout(area());

        assert_eq!(layout.links.len(), 4);

        for band in &layout.links {
            assert!(band.source < layout.nodes.len());
            assert!(band.target < layout.nodes.len());
            assert!(band.thickness > 0.0);
        }
    }

    #[test]
    fn a_node_with_no_links_is_placed_but_flat() {
        let with_orphan = vec![
            SankeyNode::new("A"),
            SankeyNode::new("B"),
            SankeyNode::new("Orphan"),
        ];
        let links = vec![SankeyLink::new(0, 1, 10.0)];

        let program = program_of(SankeyChart::new(with_orphan, links));
        let layout = program.layout(area());

        // The orphan is still in the layout, just with a minimal height.
        assert_eq!(layout.nodes.len(), 3);
        assert!(layout.nodes[2].height >= 1.0);
    }

    #[test]
    fn an_empty_chart_renders_without_panicking() {
        let no_nodes: iced::Element<'_, Message, Theme> =
            SankeyChart::new(Vec::new(), Vec::new()).into_element();
        drop(no_nodes);

        let no_links: iced::Element<'_, Message, Theme> =
            SankeyChart::new(nodes(), Vec::new()).into_element();
        drop(no_links);

        let no_nodes_with_links: iced::Element<'_, Message, Theme> =
            SankeyChart::new(Vec::new(), links()).into_element();
        drop(no_nodes_with_links);
    }

    #[test]
    fn a_degenerate_area_yields_no_layout() {
        let program = program_of(SankeyChart::new(nodes(), links()));

        for bad in [
            Rectangle {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 100.0,
            },
            Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 0.0,
            },
        ] {
            assert!(program.layout(bad).is_empty(), "a {bad:?} area has no room");
        }
    }

    #[test]
    fn a_single_node_renders() {
        let element: iced::Element<'_, Message, Theme> =
            SankeyChart::new(vec![SankeyNode::new("Only")], Vec::new()).into_element();
        drop(element);
    }

    #[test]
    fn every_option_combination_renders() {
        let element: iced::Element<'_, Message, Theme> = SankeyChart::new(nodes(), links())
            .align(SankeyAlign::Left)
            .value_scale(SankeyValueScale::Sqrt)
            .node_width(24.0)
            .node_padding(16.0)
            .link_opacity(0.6)
            .iterations(12)
            .labels(false)
            .tooltip(false)
            .height(400.0)
            .tone(Tone::Success)
            .into_element();
        drop(element);
    }

    #[test]
    fn the_iteration_count_is_clamped() {
        // Zero passes is legitimate (a raw layout); thousands would stall a
        // frame, so the count is bounded.
        assert_eq!(
            SankeyChart::new(nodes(), links()).iterations(0).iterations,
            0
        );
        assert_eq!(
            SankeyChart::new(nodes(), links())
                .iterations(9_999)
                .iterations,
            32
        );
    }

    #[test]
    fn nodes_get_distinct_colors() {
        let program = program_of(SankeyChart::new(nodes(), links()));
        let theme = Theme::light();

        let colors: Vec<Color> = (0..5)
            .map(|index| program.color_of(index, &theme))
            .collect();

        for (i, left) in colors.iter().enumerate() {
            for (j, right) in colors.iter().enumerate() {
                if i != j {
                    assert_ne!(left, right, "node {i} and {j} share a colour");
                }
            }
        }
    }

    #[test]
    fn a_node_with_an_explicit_color_keeps_it() {
        let program = program_of(SankeyChart::new(
            vec![SankeyNode::new("A").color(Color::WHITE)],
            Vec::new(),
        ));

        assert_eq!(program.color_of(0, &Theme::light()), Color::WHITE);
    }

    #[test]
    fn node_metadata_is_reported() {
        let node = SankeyNode::new("Coal").tone(Tone::Warning);
        assert_eq!(node.label(), "Coal");
    }
}
