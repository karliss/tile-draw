use std::ops::Mul;
use std::sync::{Arc, Mutex};
use std::thread::current;
use std::vec;

use crate::tiling::*;
use egui::{emath, Id, Rect};
use kurbo::{Affine, BezPath, Point, Shape};
use whiskers::prelude::egui::emath::RectTransform;
use whiskers::prelude::egui::epaint::PathShape;
use whiskers::prelude::egui::{epaint, Color32, Painter, Pos2, Response, Sense, Stroke, Vec2};
use whiskers::widgets::Widget;
use whiskers::{prelude::*, register_widget_ui};

#[derive(Default)]
pub struct TilingEditorWidget {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    Move,
    Rotate,
    MoveRotate,
    ScaleRotate,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragMode {
    None,
    Move,
    Rotate,
    ScaleRotate,
}

/*#[derive(Clone, Debug)]
enum Selection {
    None,
    //Points { shape: usize, corners: Vec<usize> },
    Shapes { shapes: Vec<usize> },
}*/



struct WindowState {
    open: bool,
    current_tile: usize,
    anchor: Option<SubshapeCorner>,
    draw_transform: RectTransform,
    tool: Tool,
    shape_selection: Vec<usize>,
    drag_transforms: Vec<Affine>,
    drag_start_p: Pos2,
    drag_activated: bool,
    drag_grabp: Option<SubshapeCorner>,
    snap: bool,
    last_snap_pint: Option<Pos2>,
    drag_mode: DragMode,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            open: Default::default(),
            current_tile: Default::default(),
            anchor: None,
            draw_transform: RectTransform::identity(egui::Rect::ZERO),
            tool: Tool::Select,
            shape_selection: Vec::new(),
            drag_transforms: Vec::new(),
            drag_start_p: Pos2::ZERO,
            drag_activated: false,
            snap: true,
            last_snap_pint: None,
            drag_grabp: None,
            drag_mode: DragMode::None,
        }
    }
}

/*fn as_points(tile: &Tile, tr: &RectTransform) -> Vec<Pos2> {
    tile.corners
        .iter()
        .map(|p| tr * (p.x as f32, p.y as f32).into())
        .collect()
}*/

fn to_point(p: Pos2) -> kurbo::Point {
    return Point::new(p.x.into(), p.y.into());
}

fn to_pos(p: kurbo::Point) -> Pos2 {
    return Pos2::new(p.x as f32, p.y as f32);
}

fn to_tile_vec(p: egui::Vec2) -> kurbo::Vec2 {
    kurbo::Vec2 {
        x: p.x.into(),
        y: p.y.into(),
    }
}

fn as_points(tile: &Tile, placement: &Affine, tr: &RectTransform) -> Vec<Pos2> {
    tile.corners
        .iter()
        .map(|p| {
            let p = *placement * *p;
            tr * (p.x as f32, p.y as f32).into()
        })
        .collect()
}

fn rough_bounds(path: &BezPath, transform: &RectTransform) -> Rect {
    let bbox = path.bounding_box();
    let mut res = Rect::NOTHING;
    res.extend_with(transform.transform_pos(to_pos(bbox.origin())));
    let p2 = Point::new(bbox.x1, bbox.y1);
    res.extend_with(transform.transform_pos(to_pos(p2)));
    return res;
}

const DRAG_START: f64 = 5.0;
const SNAP_DISTANCE: f64 = 0.04;

impl WindowState {
    fn display_shapes(
        &mut self,
        ui: &mut egui::Ui,
        value: &mut TilingStep,
        resp_painter: &(Response, Painter),
    ) {
        let mut clicked_something = false;

        let current_rule = value.rules[self.current_tile].clone();
        let mouse_pos = ui
            .input(|inp| inp.pointer.hover_pos())
            .unwrap_or(Pos2::new(0.0, 0.0));
        let draw_mouse_pos = to_point(self.draw_transform.inverse().transform_pos(mouse_pos));

        for (j, shape) in current_rule.result.iter().enumerate() {
            if self.shape_selection.contains(&j) {
                continue;
            }
            self.process_corners(value, ui, resp_painter, &current_rule, j, shape);
        }
        //if let Selection::Shapes { shapes } = self.selection.clone() {
        for j in &self.shape_selection.clone() {
            let shape = &current_rule.result[*j];
            self.process_corners(value, ui, resp_painter, &current_rule, *j, shape);
        }
        //}
        let (response, painter) = resp_painter;

        for (j, shape) in current_rule.result.iter().enumerate() {
            let tile = &value.rules[shape.tile_id].tile;
            let id = response.id.with("subtile").with(j);
            let positioned_tile = shape.transform * value.rules[shape.tile_id].tile.to_path();
            let hovered = positioned_tile.contains(draw_mouse_pos);
            let resp = ui.interact_with_hovered(
                rough_bounds(&positioned_tile, &self.draw_transform),
                hovered,
                id,
                Sense::drag(),
            );

            let shift = ui.input(|x| x.modifiers.shift);
            if resp.clicked() {
                self.update_tile_selection(j, shift);
                clicked_something = true;
            }

            if resp.drag_started() {
                self.drag_transforms.clear();
                let mut maybe_drag = true;
                if !self.is_selected(j) {
                    if !shift {
                        self.shape_selection = vec![j]
                    } else {
                        maybe_drag = false;
                    }
                }
                if maybe_drag {
                    self.start_drag(
                        value,
                        ui,
                        resp_painter,
                        &current_rule,
                        None,
                        resp.interact_pointer_pos().unwrap_or_default(),
                    );
                }
            }
            if resp.dragged() && self.drag_transforms.len() > 0 && self.drag_mode == DragMode::Move
            {
                self.process_drag(value, ui, resp_painter, &current_rule, &resp, shift)
            }
        }

        if response.clicked() && !clicked_something {
            self.shape_selection.clear();
            self.anchor = None;
        }
    }

    fn is_selected(&self, tile: usize) -> bool {
        self.shape_selection.contains(&tile)
    }

    fn start_drag(
        &mut self,
        value: &mut TilingStep,
        ui: &mut egui::Ui,
        resp_painter: &(Response, Painter),
        current_rule: &TilingRule,
        drag_corner: Option<SubshapeCorner>,
        drag_startp: Pos2,
    ) {
        self.drag_transforms.clear();
        let shapes = &self.shape_selection;

        for shape in shapes {
            self.drag_transforms
                .push(current_rule.result[*shape].transform);
        }
        self.drag_start_p = drag_startp; //resp_painter.0.interact_pointer_pos().unwrap_or_default();
        self.drag_grabp = drag_corner;
        self.drag_mode = match self.tool {
            Tool::Move => DragMode::Move,
            Tool::Rotate if drag_corner.is_some() => DragMode::Rotate,
            Tool::MoveRotate => {
                if drag_corner.is_some() {
                    DragMode::Rotate
                } else {
                    DragMode::Move
                }
            }
            Tool::ScaleRotate if drag_corner.is_some() => DragMode::ScaleRotate,
            _ => DragMode::None,
        };
        if self.drag_mode == DragMode::Rotate {
            // choose anchor if needed
            if self.anchor.is_none() || self.anchor == drag_corner {
                let mut best: Option<SubshapeCorner> = None;
                let mut best_distance = 0f64;
                let drag_corner = drag_corner.unwrap_or(SubshapeCorner {
                    subshape: 0,
                    corner: 0,
                });
                let drag_corner_shape = &current_rule.result[drag_corner.subshape];

                let drag_corner_pos = drag_corner_shape.transform
                    * value.rules[drag_corner_shape.tile_id].tile.corners[drag_corner.corner];
                // TODO: use any snapped point if available otherwise furthest from grap point

                for subshape in shapes {
                    let placed_shape = &current_rule.result[*subshape];
                    let shape_info = &value.rules[placed_shape.tile_id];
                    for (corner_i, corner) in shape_info.tile.corners.iter().enumerate() {
                        let placed_corner = placed_shape.transform * *corner;
                        let dis = (drag_corner_pos - placed_corner).length_squared();
                        if dis > best_distance {
                            best_distance = dis;
                            best = Some(SubshapeCorner {
                                subshape: *subshape,
                                corner: corner_i,
                            });
                        }
                    }
                }
                self.anchor = best;
            }
        }

        self.drag_activated = false;
    }

    fn process_drag(
        &mut self,
        value: &mut TilingStep,
        ui: &mut egui::Ui,
        resp_painter: &(Response, Painter),
        current_rule: &TilingRule,
        resp: &Response,
        shift: bool,
    ) {
        let p2 = resp.interact_pointer_pos().unwrap_or_default();
        let transform = self.draw_transform.inverse();
        let mouse_movement = p2 - self.drag_start_p;

        let current_drag_target = to_point(transform.transform_pos(p2));

        let movement_draw =
        transform.transform_pos(p2) - transform.transform_pos(self.drag_start_p);

        if mouse_movement.length() > DRAG_START as f32 {
            self.drag_activated = true;
        }

        if !self.drag_activated || self.drag_transforms.is_empty() {
            return;
        }

        let can_snap = self.snap && !shift;

        let shapes = &self.shape_selection;

        match self.drag_mode {
            DragMode::None => {}
            DragMode::Move => {
                let current_rule: &mut TilingRule = &mut value.rules[self.current_tile];
                for (i, shape) in shapes.iter().enumerate() {
                    current_rule.result[*shape].transform =
                        self.drag_transforms[i].then_translate(to_tile_vec(movement_draw));
                }
                if can_snap {
                    let snap_points = value.snap_targets(self.current_tile, shapes);
                    let movable_points = value.rule_points(self.current_tile, shapes);
                    let mut best: Option<(Point, Point)> = None;
                    let mut best_distance = 0f64;
                    for targets in &snap_points {
                        for movable_point in &movable_points {
                            let dis = (*targets - *movable_point).length_squared();
                            if dis < (SNAP_DISTANCE * SNAP_DISTANCE)
                                && (best.is_none() || dis < best_distance)
                            {
                                best_distance = dis;
                                best = Some((*targets, *movable_point));
                            }
                        }
                    }
                    let s1 = snap_points.len();
                    let s2 = movable_points.len();
                    if let Some((t, f)) = best {
                        resp_painter.1.circle(
                            self.draw_transform * to_pos(t),
                            10.0,
                            Color32::TRANSPARENT,
                            Stroke::new(1.0, Color32::BLACK),
                        );
                        let current_rule = &mut value.rules[self.current_tile];
                        let movement = t - f;
                        for shape in shapes.iter() {
                            current_rule.result[*shape].transform = current_rule.result[*shape]
                                .transform
                                .then_translate(movement);
                        }
                    }
                }
            }
            DragMode::Rotate => {
                let drag_corner = self.drag_grabp.unwrap_or(SubshapeCorner{subshape: 0, corner: 0});
                let drag_corner_pos =
                    value.subshape_point(self.current_tile,  self.drag_grabp).unwrap().1;

                let drag_corner_pos = self.drag_transforms[(self
                    .shape_selection
                    .iter()
                    .position(|x| *x == drag_corner.subshape)
                    .unwrap_or(0))]
                    * drag_corner_pos;

                let anchor = value.subshape_point(self.current_tile, self.anchor).unwrap();
                let anchor_p = anchor.0.transform * anchor.1;

                

                let angle = (current_drag_target - anchor_p).angle() - (drag_corner_pos - anchor_p).angle();
                //let drag_corner_pos = drag_corner_shape.transform *  ;

                let current_rule: &mut TilingRule = &mut value.rules[self.current_tile];
                for (i, shape) in shapes.iter().enumerate() {
                    current_rule.result[*shape].transform =
                        self.drag_transforms[i].then_rotate_about(angle, anchor_p);
                }
            }
            DragMode::ScaleRotate => {
                eprintln!("not implemented");
            }
        }
    }

    fn process_corners(
        &mut self,
        value: &mut TilingStep,
        ui: &mut egui::Ui,
        resp_painter: &(Response, Painter),
        current_rule: &TilingRule,
        j: usize,
        shape: &TilePlacement,
    ) {
        let tile = &value.rules[shape.tile_id].tile;

        let points = as_points(tile, &shape.transform, &self.draw_transform);
        let shift = ui.input(|x| x.modifiers.shift);
        for (i, p) in points.iter().enumerate() {
            let point_rect = Rect::from_center_size(*p, egui::Vec2::new(8.0, 8.0));
            let point_resp = ui.interact(
                point_rect,
                resp_painter.0.id.with("point").with(j).with(i),
                Sense::drag(),
            );
            if point_resp.hovered() {
                resp_painter.1.circle(
                    *p,
                    7.0,
                    Color32::TRANSPARENT,
                    Stroke::new(1.0, Color32::GREEN),
                );
            }
            if point_resp.clicked() {
                self.anchor = Some(SubshapeCorner {
                    subshape: j,
                    corner: i,
                });
            }
            if point_resp.drag_started() {
                if self.shape_selection.contains(&j) {
                    self.start_drag(
                        value,
                        ui,
                        resp_painter,
                        current_rule,
                        Some(SubshapeCorner {
                            subshape: j,
                            corner: i,
                        }),
                        point_resp.interact_pointer_pos().unwrap_or_default(),
                    )
                }
            } else if point_resp.dragged() {
                self.process_drag(value, ui, resp_painter, current_rule, &point_resp, shift)
            } else if point_resp.drag_released() {
                self.drag_mode = DragMode::None;
            }

            match &self.anchor {
                Some(corner) if corner.subshape == j && i == corner.corner => {
                    resp_painter.1.line_segment(
                        [*p + Vec2::new(-8.0, -8.0), *p - Vec2::new(-8.0, -8.0)],
                        Stroke::new(1.0, Color32::DARK_BLUE),
                    );
                    resp_painter.1.line_segment(
                        [*p + Vec2::new(8.0, -8.0), *p - Vec2::new(8.0, -8.0)],
                        Stroke::new(1.0, Color32::DARK_BLUE),
                    );
                }
                _ => {}
            }
        }

        let mut stroke = Stroke::new(1.0, Color32::BLACK);
        if self.shape_selection.contains(&j) {
            stroke.color = Color32::GREEN;
        }

        let shape = egui::Shape::closed_line(points, stroke);
        ui.painter().add(shape);
    }

    fn update_tile_selection(&mut self, tile: usize, shift: bool) {
        if !shift {
            self.shape_selection = vec![tile];
            self.anchor = None;
        } else {
            if self.shape_selection.contains(&tile) {
                // remove from seleciton tile
                self.shape_selection.retain(|x| *x != tile);
                if self.anchor.is_some_and(|x| x.subshape == tile) {
                    self.anchor = None;
                }
            } else {
                // add to selectio
                self.shape_selection.push(tile);
            }
        }
    }

    fn tiling_editor_window(&mut self, ui: &mut egui::Ui, value: &mut TilingStep, window_id: Id) {
        let ctx = ui.ctx();

        let mut open = self.open;
        egui::Window::new("My Window")
            .id(window_id)
            .open(&mut open)
            .show(ctx, |ui| {
                let selected_tile = self.current_tile;
                egui::SidePanel::left("tileedit_left")
                    .resizable(true)
                    .default_width(150.0)
                    .width_range(80.0..=200.0)
                    .show_inside(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Left Panel");
                        });
                        let rule_selection = egui::ComboBox::from_label("Tile")
                            .selected_text(format!("{selected_tile}"))
                            .show_ui(ui, |ui| {
                                //ui.style_mut().wrap = Some(false);
                                //ui.set_min_width(60.0);
                                for (i, rule) in value.rules.iter().enumerate() {
                                    ui.selectable_value(&mut self.current_tile, i, format!("{i}"));
                                }
                            });

                        if rule_selection.response.changed() {
                            self.shape_selection.clear();
                        }

                        let shift = ui.input(|x| x.modifiers.shift);
                        ui.add_enabled_ui(!shift, |ui| {
                            ui.checkbox(&mut self.snap, "Snap");
                        });

                        ui.radio_value(&mut self.tool, Tool::Select, "Select");
                        ui.radio_value(&mut self.tool, Tool::Move, "Move");
                        ui.radio_value(&mut self.tool, Tool::Rotate, "Rotate");
                        ui.radio_value(&mut self.tool, Tool::MoveRotate, "Move+Rotate");
                        /*egui::ScrollArea::vertical().show(ui, |ui| {

                        });*/
                    });

                egui::SidePanel::right("tileedit_right")
                    .resizable(true)
                    .default_width(150.0)
                    .width_range(80.0..=200.0)
                    .show_inside(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Right Panel");
                        });
                        egui::ScrollArea::vertical().show(ui, |ui| {});
                    });

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Central Panel");
                    });
                    egui::ScrollArea::both().show(ui, |ui| {
                        let available_space = ui.available_size();
                        //let space = egui::Vec2::new(available_space.min_elem(), available_space.min_elem());
                        //let (_id, rect) = ui.allocate_space(available_space);
                        let (response, painter) =
                            ui.allocate_painter(available_space, Sense::click());

                        let target_rect = if available_space.x > available_space.y {
                            let xs = 0.5 * 4.0 * available_space.x / available_space.y;
                            Rect::from_x_y_ranges(-xs..=xs, 2.0..=-2.0)
                        } else {
                            let ys = 0.5 * 4.0 * available_space.y / available_space.x;
                            Rect::from_x_y_ranges(-2.0..=2.0, ys..=-ys)
                        };

                        let to_screen = emath::RectTransform::from_to(target_rect, response.rect);
                        self.draw_transform = to_screen.clone();

                        ui.painter().arrow(
                            to_screen * Pos2::new(-2.0, 0.0),
                            egui::Vec2::new(4.0, 0.0).mul(to_screen.scale()),
                            egui::Stroke::new(1.0, Color32::GRAY),
                        );
                        ui.painter().arrow(
                            to_screen * Pos2::new(0.0, -2.0),
                            egui::Vec2::new(0.0, 4.0).mul(to_screen.scale()),
                            egui::Stroke::new(1.0, Color32::GRAY),
                        );

                        if !(0..=value.rules.len()).contains(&(self.current_tile)) {
                            return;
                        }
                        let rule = &value.rules[self.current_tile];
                        let points = as_points(&rule.tile, &Affine::IDENTITY, &to_screen);

                        painter.add(egui::Shape::closed_line(
                            points,
                            Stroke::new(4.0, Color32::LIGHT_BLUE),
                        ));

                        self.display_shapes(ui, value, &(response, painter));
                    });
                });
            });
        self.open = open;
    }
}

impl Widget<TilingStep> for TilingEditorWidget {
    fn ui(&self, ui: &mut egui::Ui, label: &str, value: &mut TilingStep) -> bool {
        let window_id = Id::new("My window");
        let ctx = ui.ctx();
        let window_data: Arc<Mutex<WindowState>> =
            ctx.memory(|mem| mem.data.get_temp(window_id).unwrap_or_default());
        {
            let mut window_state = window_data.lock().unwrap();
            window_state.tiling_editor_window(ui, value, window_id);
            if ui.button("Edit tiling").clicked() {
                window_state.open = true;
            }
        }
        ui.ctx().memory_mut(|mem| {
            mem.data.insert_temp(window_id, window_data);
        });

        false
    }
}

register_widget_ui!(TilingStep, TilingEditorWidget);
