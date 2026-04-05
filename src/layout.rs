use crate::css::{Unit, Value};
use crate::style::{PropertyMap, StyledNode};

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    pub fn total_width(&self) -> f32 {
        self.content.width
            + self.padding.left
            + self.padding.right
            + self.border.left
            + self.border.right
            + self.margin.left
            + self.margin.right
    }

    pub fn total_height(&self) -> f32 {
        self.content.height
            + self.padding.top
            + self.padding.bottom
            + self.border.top
            + self.border.bottom
            + self.margin.top
            + self.margin.bottom
    }

    pub fn margin_box(&self) -> Rect {
        Rect {
            x: self.content.x - self.padding.left - self.border.left - self.margin.left,
            y: self.content.y - self.padding.top - self.border.top - self.margin.top,
            width: self.total_width(),
            height: self.total_height(),
        }
    }

    pub fn border_box(&self) -> Rect {
        Rect {
            x: self.content.x - self.padding.left - self.border.left,
            y: self.content.y - self.padding.top - self.border.top,
            width: self.content.width
                + self.padding.left
                + self.padding.right
                + self.border.left
                + self.border.right,
            height: self.content.height
                + self.padding.top
                + self.padding.bottom
                + self.border.top
                + self.border.bottom,
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum BoxType {
    Block,
    Inline,
    Anonymous,
}


#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub dimensions: Dimensions,
    pub box_type: BoxType,
    pub children: Vec<LayoutBox>,
    pub styles: PropertyMap,
    pub text: Option<String>,
}

impl LayoutBox {
    fn new(box_type: BoxType) -> Self {
        LayoutBox {
            dimensions: Dimensions::default(),
            box_type,
            children: vec![],
            styles: PropertyMap::new(),
            text: None,
        }
    }
}

pub struct LayoutEngine {
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl LayoutEngine {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        LayoutEngine { viewport_width, viewport_height }
    }

    pub fn layout(&self, styled_node: &StyledNode) -> LayoutBox {
        let mut root = build_layout_box(styled_node);
        root.dimensions.content.x = 0.0;
        root.dimensions.content.y = 0.0;
        root.dimensions.content.width = self.viewport_width;
        layout_box(&mut root, self.viewport_width);
        root
    }
}

fn display_type(styled_node: &StyledNode) -> BoxType {
    match styled_node.styles.get("display") {
        Some(Value::Keyword(kw)) if kw == "inline" => BoxType::Inline,
        Some(Value::Keyword(kw)) if kw == "none" => {
            BoxType::Inline
        }
        _ => BoxType::Block,
    }
}

fn build_layout_box(styled_node: &StyledNode) -> LayoutBox {
    let box_type = display_type(styled_node);
    let mut layout_box = LayoutBox::new(box_type);
    layout_box.styles = styled_node.styles.clone();

    if let crate::dom::NodeType::Text(ref t) = styled_node.node.node_type {
        layout_box.text = Some(t.clone());
    }

    for child in &styled_node.children {
        let child_box = build_layout_box(child);
        layout_box.children.push(child_box);
    }

    layout_box
}

fn get_px(styled_node_styles: &crate::style::PropertyMap, prop: &str) -> f32 {
    match styled_node_styles.get(prop) {
        Some(Value::Length(n, Unit::Px)) => *n,
        _ => 0.0,
    }
}

fn apply_box_model(dims: &mut Dimensions, styles: &crate::style::PropertyMap) {
    dims.padding.top    = get_px(styles, "padding-top").max(get_px(styles, "padding"));
    dims.padding.bottom = get_px(styles, "padding-bottom").max(get_px(styles, "padding"));
    dims.padding.left   = get_px(styles, "padding-left").max(get_px(styles, "padding"));
    dims.padding.right  = get_px(styles, "padding-right").max(get_px(styles, "padding"));

    dims.border.top    = get_px(styles, "border-top-width").max(get_px(styles, "border-width"));
    dims.border.bottom = get_px(styles, "border-bottom-width").max(get_px(styles, "border-width"));
    dims.border.left   = get_px(styles, "border-left-width").max(get_px(styles, "border-width"));
    dims.border.right  = get_px(styles, "border-right-width").max(get_px(styles, "border-width"));

    dims.margin.top    = get_px(styles, "margin-top").max(get_px(styles, "margin"));
    dims.margin.bottom = get_px(styles, "margin-bottom").max(get_px(styles, "margin"));
    dims.margin.left   = get_px(styles, "margin-left").max(get_px(styles, "margin"));
    dims.margin.right  = get_px(styles, "margin-right").max(get_px(styles, "margin"));
}

fn layout_box(b: &mut LayoutBox, container_width: f32) {
    match b.box_type {
        BoxType::Block => layout_block(b, container_width),
        BoxType::Inline | BoxType::Anonymous => layout_inline_container(b, container_width),
    }
}


fn layout_block(b: &mut LayoutBox, container_width: f32) {
    let used_horizontal = b.dimensions.padding.left
        + b.dimensions.padding.right
        + b.dimensions.border.left
        + b.dimensions.border.right
        + b.dimensions.margin.left
        + b.dimensions.margin.right;

    b.dimensions.content.width = (container_width - used_horizontal).max(0.0);

    if b.children.is_empty() {
        return;
    }

    let origin_y = b.dimensions.content.y;
    let origin_x = b.dimensions.content.x
        + b.dimensions.padding.left
        + b.dimensions.border.left;

    let mut offset_y = 0.0_f32;
    let mut prev_margin_bottom = 0.0_f32;

    for child in &mut b.children {
        let child_margin_top = child.dimensions.margin.top;
        let collapsed = prev_margin_bottom.max(child_margin_top);
        let gap = collapsed;

        child.dimensions.content.x = origin_x
            + child.dimensions.margin.left
            + child.dimensions.border.left
            + child.dimensions.padding.left;
        child.dimensions.content.y = origin_y
            + offset_y
            + gap
            + child.dimensions.border.top
            + child.dimensions.padding.top;

        layout_box(child, b.dimensions.content.width);

        let child_outer_h = child.dimensions.content.height
            + child.dimensions.padding.top
            + child.dimensions.padding.bottom
            + child.dimensions.border.top
            + child.dimensions.border.bottom;

        offset_y += gap + child_outer_h;
        prev_margin_bottom = child.dimensions.margin.bottom;
    }

    offset_y += prev_margin_bottom;

    b.dimensions.content.height = offset_y.max(0.0);
}

fn layout_inline_container(b: &mut LayoutBox, container_width: f32) {
    let available_width = container_width
        - b.dimensions.padding.left
        - b.dimensions.padding.right
        - b.dimensions.border.left
        - b.dimensions.border.right;

    if b.children.is_empty() {
        return;
    }

    let line_start_x = b.dimensions.content.x
        + b.dimensions.padding.left
        + b.dimensions.border.left;
    let origin_y = b.dimensions.content.y
        + b.dimensions.padding.top
        + b.dimensions.border.top;

    let mut cursor_x = line_start_x;
    let mut cursor_y = origin_y;
    let mut line_height = 0.0_f32;

    for child in &mut b.children {
        let child_w = child.dimensions.content.width
            + child.dimensions.padding.left
            + child.dimensions.padding.right
            + child.dimensions.border.left
            + child.dimensions.border.right
            + child.dimensions.margin.left
            + child.dimensions.margin.right;

        if child_w > 0.0
            && cursor_x + child_w > line_start_x + available_width
            && cursor_x > line_start_x
        {
            cursor_x = line_start_x;
            cursor_y += line_height;
            line_height = 0.0;
        }

        child.dimensions.content.x = cursor_x
            + child.dimensions.margin.left
            + child.dimensions.border.left
            + child.dimensions.padding.left;
        child.dimensions.content.y = cursor_y
            + child.dimensions.margin.top
            + child.dimensions.border.top
            + child.dimensions.padding.top;

        layout_box(child, child.dimensions.content.width.max(available_width));

        let child_h = child.dimensions.content.height
            + child.dimensions.padding.top
            + child.dimensions.padding.bottom
            + child.dimensions.border.top
            + child.dimensions.border.bottom
            + child.dimensions.margin.top
            + child.dimensions.margin.bottom;

        if child_h > line_height {
            line_height = child_h;
        }

        cursor_x += child_w;
    }

    b.dimensions.content.height = (cursor_y + line_height - origin_y).max(0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dims(
        content_w: f32, content_h: f32,
        pad: f32, border: f32, margin: f32,
    ) -> Dimensions {
        Dimensions {
            content: Rect { x: 0.0, y: 0.0, width: content_w, height: content_h },
            padding: EdgeSizes { left: pad, right: pad, top: pad, bottom: pad },
            border: EdgeSizes { left: border, right: border, top: border, bottom: border },
            margin: EdgeSizes { left: margin, right: margin, top: margin, bottom: margin },
        }
    }

    #[test]
    fn test_total_width_content_only() {
        let d = make_dims(100.0, 50.0, 0.0, 0.0, 0.0);
        assert_eq!(d.total_width(), 100.0);
    }

    #[test]
    fn test_total_width_with_padding_border_margin() {
        let d = make_dims(100.0, 50.0, 10.0, 2.0, 5.0);
        assert_eq!(d.total_width(), 134.0);
    }

    #[test]
    fn test_total_height_with_padding_border_margin() {
        let d = make_dims(100.0, 50.0, 10.0, 2.0, 5.0);
        assert_eq!(d.total_height(), 84.0);
    }

    #[test]
    fn test_border_box_dimensions() {
        let d = make_dims(100.0, 50.0, 10.0, 2.0, 5.0);
        let bb = d.border_box();
        assert_eq!(bb.width, 124.0);
        assert_eq!(bb.height, 74.0);
    }

    #[test]
    fn test_margin_box_dimensions() {
        let d = make_dims(100.0, 50.0, 10.0, 2.0, 5.0);
        let mb = d.margin_box();
        assert_eq!(mb.width, 134.0);
        assert_eq!(mb.height, 84.0);
    }


    #[test]
    fn test_block_children_stack_vertically() {
        let mut child1 = LayoutBox::new(BoxType::Block);
        child1.dimensions.content.height = 50.0;

        let mut child2 = LayoutBox::new(BoxType::Block);
        child2.dimensions.content.height = 50.0;

        let mut parent = LayoutBox::new(BoxType::Block);
        parent.dimensions.content.width = 200.0;
        parent.dimensions.content.x = 0.0;
        parent.dimensions.content.y = 0.0;
        parent.children.push(child1);
        parent.children.push(child2);

        layout_block(&mut parent, 200.0);

        assert_eq!(parent.children[0].dimensions.content.y, 0.0);
        assert_eq!(parent.children[1].dimensions.content.y, 50.0);
        assert_eq!(parent.dimensions.content.height, 100.0);
    }

    #[test]
    fn test_block_no_horizontal_overlap() {
        let mut child1 = LayoutBox::new(BoxType::Block);
        child1.dimensions.content.height = 30.0;

        let mut child2 = LayoutBox::new(BoxType::Block);
        child2.dimensions.content.height = 40.0;

        let mut parent = LayoutBox::new(BoxType::Block);
        parent.dimensions.content.width = 300.0;
        parent.children.push(child1);
        parent.children.push(child2);

        layout_block(&mut parent, 300.0);

        let c1 = &parent.children[0];
        let c2 = &parent.children[1];
        assert_eq!(c1.dimensions.content.x, 0.0);
        assert_eq!(c2.dimensions.content.x, 0.0);
        assert!(c2.dimensions.content.y >= c1.dimensions.content.y + c1.dimensions.content.height);
    }

    #[test]
    fn test_block_margin_collapse() {
        let mut child1 = LayoutBox::new(BoxType::Block);
        child1.dimensions.content.height = 50.0;
        child1.dimensions.margin.bottom = 20.0;

        let mut child2 = LayoutBox::new(BoxType::Block);
        child2.dimensions.content.height = 50.0;
        child2.dimensions.margin.top = 10.0;

        let mut parent = LayoutBox::new(BoxType::Block);
        parent.dimensions.content.width = 200.0;
        parent.dimensions.content.y = 0.0;
        parent.children.push(child1);
        parent.children.push(child2);

        layout_block(&mut parent, 200.0);

        assert_eq!(parent.children[0].dimensions.content.y, 0.0);
        assert_eq!(parent.children[1].dimensions.content.y, 70.0);
    }

    #[test]
    fn test_inline_children_flow_horizontally() {
        let mut child1 = LayoutBox::new(BoxType::Inline);
        child1.dimensions.content.width = 50.0;
        child1.dimensions.content.height = 20.0;

        let mut child2 = LayoutBox::new(BoxType::Inline);
        child2.dimensions.content.width = 50.0;
        child2.dimensions.content.height = 20.0;

        let mut parent = LayoutBox::new(BoxType::Inline);
        parent.dimensions.content.width = 200.0;
        parent.dimensions.content.x = 0.0;
        parent.dimensions.content.y = 0.0;
        parent.children.push(child1);
        parent.children.push(child2);

        layout_inline_container(&mut parent, 200.0);

        assert_eq!(parent.children[0].dimensions.content.y, 0.0);
        assert_eq!(parent.children[1].dimensions.content.y, 0.0);
        assert!(parent.children[1].dimensions.content.x > parent.children[0].dimensions.content.x);
    }

    #[test]
    fn test_inline_wraps_when_exceeding_container_width() {
        let mut child1 = LayoutBox::new(BoxType::Inline);
        child1.dimensions.content.width = 70.0;
        child1.dimensions.content.height = 20.0;

        let mut child2 = LayoutBox::new(BoxType::Inline);
        child2.dimensions.content.width = 70.0;
        child2.dimensions.content.height = 20.0;

        let mut parent = LayoutBox::new(BoxType::Inline);
        parent.dimensions.content.width = 100.0;
        parent.dimensions.content.x = 0.0;
        parent.dimensions.content.y = 0.0;
        parent.children.push(child1);
        parent.children.push(child2);

        layout_inline_container(&mut parent, 100.0);

        assert_eq!(parent.children[0].dimensions.content.y, 0.0);
        assert!(parent.children[1].dimensions.content.y > 0.0);
    }

    #[test]
    fn test_layout_engine_new() {
        let engine = LayoutEngine::new(800.0, 600.0);
        assert_eq!(engine.viewport_width, 800.0);
        assert_eq!(engine.viewport_height, 600.0);
    }

    #[test]
    fn test_default_rect() {
        let r = Rect::default();
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 0.0);
        assert_eq!(r.width, 0.0);
        assert_eq!(r.height, 0.0);
    }

    #[test]
    fn test_default_edge_sizes() {
        let e = EdgeSizes::default();
        assert_eq!(e.left, 0.0);
        assert_eq!(e.right, 0.0);
        assert_eq!(e.top, 0.0);
        assert_eq!(e.bottom, 0.0);
    }
}
