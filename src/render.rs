use crate::css::{Color, Value};
use crate::layout::LayoutBox;

pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Canvas {
            width,
            height,
            pixels: vec![0u8; (width * height * 4) as usize],
        }
    }

    pub fn paint_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [u8; 4]) {
        let x0 = x.max(0.0) as u32;
        let y0 = y.max(0.0) as u32;
        let x1 = (x + width).min(self.width as f32) as u32;
        let y1 = (y + height).min(self.height as f32) as u32;

        for row in y0..y1 {
            for col in x0..x1 {
                let idx = ((row * self.width + col) * 4) as usize;
                self.pixels[idx]     = color[0];
                self.pixels[idx + 1] = color[1];
                self.pixels[idx + 2] = color[2];
                self.pixels[idx + 3] = color[3];
            }
        }
    }

    pub fn paint_border(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        border_width: f32,
        color: [u8; 4],
    ) {
        if border_width <= 0.0 {
            return;
        }
        let bw = border_width;
        // Top border
        self.paint_rect(x, y, width, bw, color);
        // Bottom border
        self.paint_rect(x, y + height - bw, width, bw, color);
        // Left border
        self.paint_rect(x, y, bw, height, color);
        // Right border
        self.paint_rect(x + width - bw, y, bw, height, color);
    }

    pub fn paint_text(&mut self, x: f32, y: f32, text: &str, color: [u8; 4]) {
        let char_width = 6.0_f32;
        let char_height = 10.0_f32;
        let total_width = text.len() as f32 * char_width;
        self.paint_rect(x, y, total_width, char_height, color);
    }
}

pub struct RenderEngine {
    canvas: Canvas,
}

impl RenderEngine {
    pub fn new(width: u32, height: u32) -> Self {
        RenderEngine {
            canvas: Canvas::new(width, height),
        }
    }

    pub fn render(&mut self, layout_root: &LayoutBox) {
        self.render_box(layout_root);
    }

    pub fn get_pixels(&self) -> &[u8] {
        &self.canvas.pixels
    }

    fn render_box(&mut self, b: &LayoutBox) {
        self.render_background(b);
        self.render_borders(b);
        self.render_text(b);

        for child in &b.children {
            self.render_box(child);
        }
    }

    fn render_background(&mut self, b: &LayoutBox) {
        let color = b.styles.get("background-color")
            .or_else(|| b.styles.get("background"))
            .and_then(|v| color_from_value(v));

        if let Some(c) = color {
            let bb = b.dimensions.border_box();
            self.canvas.paint_rect(bb.x, bb.y, bb.width, bb.height, c);
        }
    }

    fn render_borders(&mut self, b: &LayoutBox) {
        let border_width = {
            let d = &b.dimensions;
            let w = d.border.top
                .max(d.border.bottom)
                .max(d.border.left)
                .max(d.border.right);
            w
        };

        if border_width <= 0.0 {
            return;
        }

        let color = b.styles.get("border-color")
            .and_then(|v| color_from_value(v))
            .unwrap_or([0, 0, 0, 255]);

        let bb = b.dimensions.border_box();
        self.canvas.paint_border(bb.x, bb.y, bb.width, bb.height, border_width, color);
    }

    fn render_text(&mut self, b: &LayoutBox) {
        let text = match &b.text {
            Some(t) if !t.trim().is_empty() => t.clone(),
            _ => return,
        };

        let color = b.styles.get("color")
            .and_then(|v| color_from_value(v))
            .unwrap_or([0, 0, 0, 255]);

        let x = b.dimensions.content.x;
        let y = b.dimensions.content.y;
        self.canvas.paint_text(x, y, &text, color);
    }
}

fn color_from_value(value: &Value) -> Option<[u8; 4]> {
    match value {
        Value::ColorValue(Color { r, g, b, a }) => Some([*r, *g, *b, *a]),
        Value::Keyword(kw) => named_color(kw),
        _ => None,
    }
}

fn named_color(name: &str) -> Option<[u8; 4]> {
    match name.to_lowercase().as_str() {
        "red"     => Some([255, 0,   0,   255]),
        "green"   => Some([0,   255, 0,   255]),
        "blue"    => Some([0,   0,   255, 255]),
        "black"   => Some([0,   0,   0,   255]),
        "white"   => Some([255, 255, 255, 255]),
        "yellow"  => Some([255, 255, 0,   255]),
        "cyan"    => Some([0,   255, 255, 255]),
        "magenta" => Some([255, 0,   255, 255]),
        "gray" | "grey" => Some([128, 128, 128, 255]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::Value;
    use crate::layout::{BoxType, Dimensions, EdgeSizes, LayoutBox, Rect};
    use std::collections::HashMap;

    fn make_box(x: f32, y: f32, w: f32, h: f32) -> LayoutBox {
        let mut b = LayoutBox {
            dimensions: Dimensions {
                content: Rect { x, y, width: w, height: h },
                padding: EdgeSizes::default(),
                border: EdgeSizes::default(),
                margin: EdgeSizes::default(),
            },
            box_type: BoxType::Block,
            children: vec![],
            styles: HashMap::new(),
            text: None,
        };
        b
    }

    #[test]
    fn test_canvas_new_all_transparent() {
        let canvas = Canvas::new(10, 10);
        assert_eq!(canvas.pixels.len(), 10 * 10 * 4);
        assert!(canvas.pixels.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_paint_rect_fills_pixels() {
        let mut canvas = Canvas::new(10, 10);
        canvas.paint_rect(0.0, 0.0, 5.0, 5.0, [255, 0, 0, 255]);
        assert_eq!(&canvas.pixels[0..4], &[255, 0, 0, 255]);
        let idx = (5 * 10 + 5) * 4;
        assert_eq!(&canvas.pixels[idx..idx + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn test_paint_rect_clips_to_canvas() {
        let mut canvas = Canvas::new(10, 10);
        canvas.paint_rect(8.0, 8.0, 100.0, 100.0, [0, 255, 0, 255]);
        let idx = (8 * 10 + 8) * 4;
        assert_eq!(&canvas.pixels[idx..idx + 4], &[0, 255, 0, 255]);
    }

    #[test]
    fn test_paint_border_draws_edges() {
        let mut canvas = Canvas::new(20, 20);
        canvas.paint_border(0.0, 0.0, 20.0, 20.0, 1.0, [255, 0, 0, 255]);
        assert_eq!(&canvas.pixels[0..4], &[255, 0, 0, 255]);
        let center_idx = (10 * 20 + 10) * 4;
        assert_eq!(&canvas.pixels[center_idx..center_idx + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn test_paint_text_marks_pixels() {
        let mut canvas = Canvas::new(100, 20);
        canvas.paint_text(0.0, 0.0, "hi", [0, 0, 255, 255]);
        assert_eq!(&canvas.pixels[0..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn test_render_background_color() {
        let mut engine = RenderEngine::new(100, 100);
        let mut b = make_box(0.0, 0.0, 50.0, 50.0);
        b.styles.insert(
            "background-color".to_string(),
            Value::ColorValue(crate::css::Color { r: 255, g: 0, b: 0, a: 255 }),
        );
        engine.render(&b);
        assert_eq!(&engine.get_pixels()[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn test_render_background_keyword() {
        let mut engine = RenderEngine::new(100, 100);
        let mut b = make_box(0.0, 0.0, 10.0, 10.0);
        b.styles.insert("background".to_string(), Value::Keyword("blue".to_string()));
        engine.render(&b);
        assert_eq!(&engine.get_pixels()[0..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn test_render_border() {
        let mut engine = RenderEngine::new(100, 100);
        let mut b = make_box(10.0, 10.0, 50.0, 50.0);
        b.dimensions.border = EdgeSizes { top: 2.0, bottom: 2.0, left: 2.0, right: 2.0 };
        b.styles.insert(
            "border-color".to_string(),
            Value::ColorValue(crate::css::Color { r: 0, g: 255, b: 0, a: 255 }),
        );
        engine.render(&b);
        let idx = (8 * 100 + 8) * 4;
        assert_eq!(&engine.get_pixels()[idx..idx + 4], &[0, 255, 0, 255]);
        let inner_idx = (15 * 100 + 15) * 4;
        assert_eq!(&engine.get_pixels()[inner_idx..inner_idx + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn test_no_background_leaves_transparent() {
        let mut engine = RenderEngine::new(50, 50);
        let b = make_box(0.0, 0.0, 50.0, 50.0);
        engine.render(&b);
        assert!(engine.get_pixels().iter().all(|&p| p == 0));
    }

    #[test]
    fn test_render_text_node() {
        let mut engine = RenderEngine::new(200, 50);
        let mut b = make_box(0.0, 0.0, 100.0, 20.0);
        b.text = Some("hello".to_string());
        b.styles.insert(
            "color".to_string(),
            Value::ColorValue(crate::css::Color { r: 0, g: 0, b: 0, a: 255 }),
        );
        engine.render(&b);
        assert_eq!(&engine.get_pixels()[0..4], &[0, 0, 0, 255]);
    }

    #[test]
    fn test_render_text_uses_color() {
        let mut engine = RenderEngine::new(200, 50);
        let mut b = make_box(0.0, 0.0, 100.0, 20.0);
        b.text = Some("hi".to_string());
        b.styles.insert("color".to_string(), Value::Keyword("red".to_string()));
        engine.render(&b);
        assert_eq!(&engine.get_pixels()[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn test_render_empty_text_no_paint() {
        let mut engine = RenderEngine::new(50, 50);
        let mut b = make_box(0.0, 0.0, 50.0, 50.0);
        b.text = Some("   ".to_string());
        engine.render(&b);
        assert!(engine.get_pixels().iter().all(|&p| p == 0));
    }

    #[test]
    fn test_render_engine_new() {
        let engine = RenderEngine::new(800, 600);
        assert_eq!(engine.get_pixels().len(), 800 * 600 * 4);
    }

    #[test]
    fn test_children_rendered_after_parent() {
        let mut engine = RenderEngine::new(100, 100);
        let mut parent = make_box(0.0, 0.0, 100.0, 100.0);
        parent.styles.insert(
            "background-color".to_string(),
            Value::ColorValue(crate::css::Color { r: 255, g: 0, b: 0, a: 255 }),
        );
        let mut child = make_box(0.0, 0.0, 50.0, 50.0);
        child.styles.insert(
            "background-color".to_string(),
            Value::ColorValue(crate::css::Color { r: 0, g: 0, b: 255, a: 255 }),
        );
        parent.children.push(child);
        engine.render(&parent);
        assert_eq!(&engine.get_pixels()[0..4], &[0, 0, 255, 255]);
        let idx = (60 * 100 + 60) * 4;
        assert_eq!(&engine.get_pixels()[idx..idx + 4], &[255, 0, 0, 255]);
    }
}
