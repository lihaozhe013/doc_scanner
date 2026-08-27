use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke,
    TextureHandle, Vec2,
};
use scanner_core::{Point, Quadrilateral};

pub struct CanvasView {
    zoom: f32,
    pan: Vec2,
    active_handle: Option<usize>,
    panning: bool,
    last_pointer: Option<Pos2>,
}

pub struct CanvasResponse {
    pub quad_changed: bool,
}

impl CanvasView {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            active_handle: None,
            panning: false,
            last_pointer: None,
        }
    }

    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.25, 4.0);
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        texture: Option<&TextureHandle>,
        image_size: Option<[u32; 2]>,
        quadrilateral: &mut Quadrilateral,
    ) -> CanvasResponse {
        let available = ui.available_size().max(Vec2::new(240.0, 240.0));
        let (canvas_rect, response) =
            ui.allocate_exact_size(available, Sense::click_and_drag());
        let painter = ui.painter_at(canvas_rect);
        painter.rect_filled(canvas_rect, 8.0, Color32::from_rgb(24, 28, 35));

        let Some([width, height]) = image_size else {
            painter.text(
                canvas_rect.center(),
                Align2::CENTER_CENTER,
                "Open an image to start scanning",
                FontId::proportional(18.0),
                Color32::from_gray(180),
            );
            return CanvasResponse {
                quad_changed: false,
            };
        };

        if width == 0 || height == 0 {
            return CanvasResponse {
                quad_changed: false,
            };
        }

        let fit_scale = (canvas_rect.width() / width as f32)
            .min(canvas_rect.height() / height as f32)
            .max(0.01);
        let scale = fit_scale * self.zoom;
        let displayed_size =
            Vec2::new(width as f32 * scale, height as f32 * scale);
        let image_rect = Rect::from_center_size(
            canvas_rect.center() + self.pan,
            displayed_size,
        );

        if let Some(texture) = texture {
            painter.image(
                texture.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            painter.rect_filled(image_rect, 0.0, Color32::from_gray(45));
            painter.text(
                image_rect.center(),
                Align2::CENTER_CENTER,
                "Preparing preview…",
                FontId::proportional(16.0),
                Color32::from_gray(180),
            );
        }

        let mut quad_changed = false;
        let handle_positions = quadrilateral
            .points
            .map(|point| image_to_screen(image_rect, point));

        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > f32::EPSILON {
                let previous_zoom = self.zoom;
                self.zoom =
                    (self.zoom * (1.0 + scroll * 0.001)).clamp(0.25, 4.0);
                if let Some(pointer) = response.hover_pos() {
                    let ratio = self.zoom / previous_zoom - 1.0;
                    self.pan += (pointer - image_rect.center()) * ratio;
                }
            }
        }

        if response.drag_started()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let (closest, distance) = handle_positions
                .iter()
                .enumerate()
                .map(|(index, position)| (index, position.distance(pointer)))
                .min_by(|left, right| {
                    left.1
                        .partial_cmp(&right.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or((0, f32::MAX));
            if distance <= 18.0 {
                self.active_handle = Some(closest);
                self.panning = false;
            } else {
                self.active_handle = None;
                self.panning = true;
            }
            self.last_pointer = Some(pointer);
        }

        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            if let Some(handle) = self.active_handle {
                let image_point = screen_to_image(image_rect, pointer);
                let clamped = Point::new(
                    image_point.x.clamp(0.0, 1.0),
                    image_point.y.clamp(0.0, 1.0),
                );
                if quadrilateral.points[handle] != clamped {
                    quadrilateral.points[handle] = clamped;
                    quad_changed = true;
                }
            } else if self.panning
                && let Some(previous) = self.last_pointer
            {
                self.pan += pointer - previous;
            }
            self.last_pointer = Some(pointer);
        }

        if !ui.input(|input| input.pointer.primary_down()) {
            self.active_handle = None;
            self.panning = false;
            self.last_pointer = None;
        }

        let handle_positions = quadrilateral
            .points
            .map(|point| image_to_screen(image_rect, point));
        painter.add(Shape::line(
            handle_positions
                .iter()
                .copied()
                .chain(std::iter::once(handle_positions[0]))
                .collect(),
            Stroke::new(2.0, Color32::from_rgb(64, 201, 155)),
        ));
        for (index, position) in handle_positions.into_iter().enumerate() {
            let selected = self.active_handle == Some(index);
            painter.circle_filled(
                position,
                if selected { 9.0 } else { 7.0 },
                if selected {
                    Color32::from_rgb(255, 210, 80)
                } else {
                    Color32::from_rgb(64, 201, 155)
                },
            );
            painter.circle_stroke(
                position,
                9.0,
                Stroke::new(1.0, Color32::WHITE),
            );
            painter.text(
                position + Vec2::new(0.0, -18.0),
                Align2::CENTER_CENTER,
                format!("P{}", index + 1),
                FontId::proportional(12.0),
                Color32::WHITE,
            );
        }

        CanvasResponse { quad_changed }
    }
}

impl Default for CanvasView {
    fn default() -> Self {
        Self::new()
    }
}

fn image_to_screen(rect: Rect, point: Point) -> Pos2 {
    Pos2::new(
        rect.left() + point.x as f32 * rect.width(),
        rect.top() + point.y as f32 * rect.height(),
    )
}

fn screen_to_image(rect: Rect, point: Pos2) -> Point {
    Point::new(
        f64::from((point.x - rect.left()) / rect.width()),
        f64::from((point.y - rect.top()) / rect.height()),
    )
}
