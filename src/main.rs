use eframe::egui;
use egui::Ui;
use egui_dock::{AllowedSplits, DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabViewer};
use rig::Rig;

mod rig;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([300.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native("Holyrig", options, Box::new(|_| Ok(Box::new(App::new()))))
}

struct AppTabViewer {
    current_index: u8,
    add_tab_request: bool,
}

impl AppTabViewer {
    fn new() -> Self {
        AppTabViewer {
            current_index: 0,
            add_tab_request: false,
        }
    }
}

impl TabViewer for AppTabViewer {
    type Tab = Rig;

    fn title(&mut self, _tab: &mut Self::Tab) -> egui::WidgetText {
        self.current_index += 1;
        format!("RIG {:?}", self.current_index).as_str().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("THIS IS {:?}", &tab.rig_type));
    }

    fn on_add(&mut self, _surface: SurfaceIndex, _node: NodeIndex) {
        self.add_tab_request = true;
    }
}

struct AppTabs {
    dock_state: DockState<Rig>,
}

impl AppTabs {
    fn new() -> Self {
        let dock_state = DockState::new(vec![Rig::default()]);
        Self { dock_state }
    }
    fn ui(&mut self, ui: &mut Ui) {
        let mut tab_viewer = AppTabViewer::new();
        DockArea::new(&mut self.dock_state)
            .show_add_buttons(true)
            .show_close_buttons(false)
            .tab_context_menus(false)
            .draggable_tabs(false)
            .show_leaf_close_all_buttons(false)
            .show_leaf_collapse_buttons(false)
            .allowed_splits(AllowedSplits::None)
            .style(Style::from_egui(ui.style().as_ref()))
            .show_inside(ui, &mut tab_viewer);

        if tab_viewer.add_tab_request {
            self.dock_state
                .main_surface_mut()
                .push_to_first_leaf(Rig::default());
            tab_viewer.add_tab_request = false;
        }
    }
}

struct App {
    tabs: AppTabs,
}

impl App {
    fn new() -> Self {
        App {
            tabs: AppTabs::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.3);
        egui::CentralPanel::default().show(ctx, |ui| self.tabs.ui(ui));
    }
}
