use crate::serial::ManagerCommand;
use eframe::egui;
use egui::{ComboBox, Grid, Ui};
use egui_dock::{AllowedSplits, DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabViewer};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::rig::{RigSettings, RigType};

pub enum GuiMessage {}

struct AppTabViewer {
    current_index: u8,
    add_tab_request: bool,
    sender: Sender<ManagerCommand>,
}

impl AppTabViewer {
    fn new(sender: Sender<ManagerCommand>) -> Self {
        AppTabViewer {
            current_index: 0,
            add_tab_request: false,
            sender,
        }
    }
}

impl TabViewer for AppTabViewer {
    type Tab = RigSettings;

    fn title(&mut self, _tab: &mut Self::Tab) -> egui::WidgetText {
        self.current_index += 1;
        format!("RIG {:?}", self.current_index).as_str().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, rig: &mut Self::Tab) {
        ui.group(|ui| {
            ui.style_mut().spacing.combo_width *= 0.75;

            Grid::new("rig_settings").num_columns(2).show(ui, |ui| {
                ui.label("RigSettings type:");
                ComboBox::from_id_salt("rig_type")
                    .selected_text(format!("{}", rig.rig_type))
                    .show_ui(ui, |ui| {
                        for rig_type in &[RigType::Unspecified, RigType::IC7300, RigType::FT891] {
                            ui.selectable_value(
                                &mut rig.rig_type,
                                *rig_type,
                                format!("{rig_type}"),
                            );
                        }
                    });
                ui.end_row();

                ui.label("Port:");
                ui.text_edit_singleline(&mut rig.port);
                ui.end_row();

                ui.label("Baud Rate:");
                ComboBox::from_id_salt("baud_rate")
                    .selected_text(format!("{}", rig.baud_rate))
                    .show_ui(ui, |ui| {
                        for rate in [1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200] {
                            ui.selectable_value(&mut rig.baud_rate, rate, format!("{rate}"));
                        }
                    });
                ui.end_row();

                ui.label("Data Bits:");
                ComboBox::from_id_salt("data_bits")
                    .selected_text(format!("{}", rig.data_bits))
                    .show_ui(ui, |ui| {
                        for bits in [7, 8] {
                            ui.selectable_value(&mut rig.data_bits, bits, format!("{bits}"));
                        }
                    });
                ui.end_row();

                ui.label("Stop Bits:");
                ComboBox::from_id_salt("stop_bits")
                    .selected_text(format!("{}", rig.stop_bits))
                    .show_ui(ui, |ui| {
                        for bits in [1, 2] {
                            ui.selectable_value(&mut rig.stop_bits, bits, format!("{bits}"));
                        }
                    });
                ui.end_row();

                ui.label("Parity:");
                ui.checkbox(&mut rig.parity, "");
                ui.end_row();

                ui.label("RTS:");
                ui.checkbox(&mut rig.rts, "");
                ui.end_row();

                ui.label("DTR:");
                ui.checkbox(&mut rig.dtr, "");
                ui.end_row();

                ui.label("Poll Interval (ms):");
                ui.add(egui::DragValue::new(&mut rig.poll_interval).range(10..=1000));
                ui.end_row();

                ui.label("Timeout (ms):");
                ui.add(egui::DragValue::new(&mut rig.timeout).range(10..=5000));
                ui.end_row();
            });

            ui.separator();

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("OK").clicked() {
                    let sender = self.sender.clone();
                    let current_index = self.current_index;
                    let tab = rig.clone();
                    tokio::task::spawn(async move {
                        sender
                            .send(ManagerCommand::ChangeSettings {
                                device_id: current_index.to_string(),
                                settings: tab.clone(),
                            })
                            .await
                            .unwrap();
                    });
                }
                if ui.button("Cancel").clicked() {}
            });
        });
    }

    fn on_add(&mut self, _surface: SurfaceIndex, _node: NodeIndex) {
        self.add_tab_request = true;
    }
}

struct AppTabs {
    dock_state: DockState<RigSettings>,
    sender: Sender<ManagerCommand>,
}

impl AppTabs {
    fn new(sender: Sender<ManagerCommand>) -> Self {
        let dock_state = DockState::new(vec![RigSettings::default()]);
        Self { dock_state, sender }
    }
    fn ui(&mut self, ui: &mut Ui) {
        let mut tab_viewer = AppTabViewer::new(self.sender.clone());

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
                .push_to_first_leaf(RigSettings::default());
            tab_viewer.add_tab_request = false;
        }
    }
}

pub struct App {
    gui_receiver: Receiver<GuiMessage>,
    tabs: AppTabs,
}

impl App {
    pub fn new(gui_receiver: Receiver<GuiMessage>, serial_sender: Sender<ManagerCommand>) -> Self {
        App {
            gui_receiver,
            tabs: AppTabs::new(serial_sender.clone()),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.3);
        egui::CentralPanel::default().show(ctx, |ui| self.tabs.ui(ui));
    }
}
