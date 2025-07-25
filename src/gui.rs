use crate::{
    rig::{BaudRate, DataBits, RigSettings, StopBits},
    serial::ManagerCommand,
};
use eframe::egui;
use egui::{ComboBox, Grid, Ui};
use egui_dock::{
    AllowedSplits, DockArea, DockState, NodeIndex, SurfaceIndex, TabViewer,
    tab_viewer::OnCloseResponse,
};
use tokio::sync::mpsc::{Receiver, Sender};

pub enum GuiMessage {
    InitialState(Vec<RigSettings>),
}

struct AppTabViewer {
    current_index: usize,
    add_tab_request: bool,
    rig_types: Vec<String>,
    sender: Sender<ManagerCommand>,
    error_message: Option<String>,
    active_tab_id: Option<usize>,
}

impl AppTabViewer {
    fn new(
        sender: Sender<ManagerCommand>,
        rig_types: Vec<String>,
        active_tab_id: Option<usize>,
    ) -> Self {
        AppTabViewer {
            current_index: 0,
            add_tab_request: false,
            rig_types,
            sender,
            error_message: None,
            active_tab_id,
        }
    }
}

impl TabViewer for AppTabViewer {
    type Tab = RigSettings;

    fn title(&mut self, _tab: &mut Self::Tab) -> egui::WidgetText {
        self.current_index += 1;
        format!("RIG {}", self.current_index).as_str().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, rig: &mut Self::Tab) {
        ui.group(|ui| {
            if let Some(error) = &self.error_message {
                ui.colored_label(egui::Color32::RED, error);
                ui.separator();
            }

            ui.style_mut().spacing.combo_width *= 0.75;

            Grid::new("rig_settings").num_columns(2).show(ui, |ui| {
                ui.label("Rig type:");
                ComboBox::from_id_salt("rig_type")
                    .selected_text(rig.rig_type.to_string())
                    .show_ui(ui, |ui| {
                        for rig_type in &self.rig_types {
                            ui.selectable_value(
                                &mut rig.rig_type,
                                rig_type.clone(),
                                rig_type.to_string(),
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
                        for rate in BaudRate::iter_rates() {
                            ui.selectable_value(&mut rig.baud_rate, rate, format!("{rate}"));
                        }
                    });
                ui.end_row();

                ui.label("Data Bits:");
                ComboBox::from_id_salt("data_bits")
                    .selected_text(format!("{}", rig.data_bits))
                    .show_ui(ui, |ui| {
                        for bits in DataBits::iter_data_bits() {
                            ui.selectable_value(&mut rig.data_bits, bits, format!("{bits}"));
                        }
                    });
                ui.end_row();

                ui.label("Stop Bits:");
                ComboBox::from_id_salt("stop_bits")
                    .selected_text(format!("{}", rig.stop_bits))
                    .show_ui(ui, |ui| {
                        for bits in [StopBits::Bits1, StopBits::Bits2] {
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
                    match rig.validate() {
                        Ok(_) => {
                            let tab = rig.clone();
                            tokio::task::spawn(async move {
                                sender
                                    .send(ManagerCommand::CreateOrUpdateDevice {
                                        settings: tab.clone(),
                                    })
                                    .await
                                    .unwrap();
                            });
                        }
                        Err(err) => {
                            self.error_message = Some(err);
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    self.error_message = None;
                }
            });
        });
    }

    fn is_closeable(&self, tab: &Self::Tab) -> bool {
        Some(tab.id) == self.active_tab_id
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> OnCloseResponse {
        let sender = self.sender.clone();
        let device_id = tab.id;

        tokio::task::spawn(async move {
            sender
                .send(ManagerCommand::RemoveDevice { device_id })
                .await
                .unwrap();
        });

        OnCloseResponse::Close
    }

    fn on_add(&mut self, _surface: SurfaceIndex, _node: NodeIndex) {
        self.add_tab_request = true;
    }
}

struct AppTabs {
    dock_state: DockState<RigSettings>,
    rig_types: Vec<String>,
    sender: Sender<ManagerCommand>,
    current_device_id: usize,
}

impl AppTabs {
    fn new(sender: Sender<ManagerCommand>, rig_types: Vec<String>) -> Self {
        let dock_state = DockState::new(vec![RigSettings::default()]);
        Self {
            dock_state,
            rig_types,
            sender,
            current_device_id: 0,
        }
    }

    fn set_tabs(&mut self, settings: Vec<RigSettings>) {
        if settings.is_empty() {
            self.current_device_id = 0;
            self.dock_state =
                DockState::new(vec![RigSettings::default().with_id(self.current_device_id)]);
        } else {
            self.current_device_id = settings.iter().map(|rig| rig.id).max().unwrap();
            self.dock_state = DockState::new(settings);
        }
    }

    fn ui(&mut self, ui: &mut Ui) {
        let active_tab_id = self
            .dock_state
            .find_active_focused()
            .map(|(_, rig)| rig.id)
            .or_else(|| {
                self.dock_state
                    .iter_leaves()
                    .next()
                    .map(|(_, rig)| rig.tabs[0].id)
            });

        let mut tab_viewer =
            AppTabViewer::new(self.sender.clone(), self.rig_types.clone(), active_tab_id);

        DockArea::new(&mut self.dock_state)
            .show_add_buttons(true)
            .show_close_buttons(true)
            .tab_context_menus(false)
            .draggable_tabs(false)
            .show_leaf_close_all_buttons(false)
            .show_leaf_collapse_buttons(false)
            .allowed_splits(AllowedSplits::None)
            .show_inside(ui, &mut tab_viewer);

        if tab_viewer.add_tab_request {
            self.current_device_id += 1;
            self.dock_state
                .main_surface_mut()
                .push_to_first_leaf(RigSettings::default().with_id(self.current_device_id));
            tab_viewer.add_tab_request = false;
        }
    }
}

pub struct App {
    gui_receiver: Receiver<GuiMessage>,
    tabs: AppTabs,
}

impl App {
    pub fn new(
        gui_receiver: Receiver<GuiMessage>,
        serial_sender: Sender<ManagerCommand>,
        rig_types: Vec<String>,
    ) -> Self {
        App {
            gui_receiver,
            tabs: AppTabs::new(serial_sender.clone(), rig_types),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(message) = self.gui_receiver.try_recv() {
            match message {
                GuiMessage::InitialState(settings) => {
                    self.tabs.set_tabs(settings);
                }
            }
        }

        ctx.set_pixels_per_point(1.3);
        egui::CentralPanel::default().show(ctx, |ui| self.tabs.ui(ui));
    }
}
