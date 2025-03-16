use relm4::{
    component,
    gtk::{
        self,
        prelude::{GridExt, WidgetExt},
    },
    ComponentParts, ComponentSender, SimpleComponent,
};

#[derive(Debug)]
pub enum RigMessage {}

pub enum RigType {
    IC7300,
    FT891,
}

pub struct Rig {
    rig_type: RigType,
    port: String,
    baud_rate: u32,
    data_bits: u8,
    parity: bool,
    stop_bits: u8,
    // true is high, false is low
    rts: bool,
    dtr: bool,
    poll_interval: u16,
    timeout: u16,
}

impl Rig {
    pub fn new() -> Self {
        Self {
            rig_type: RigType::IC7300,
            port: String::new(),
            baud_rate: 0,
            data_bits: 0,
            parity: false,
            stop_bits: 0,
            rts: false,
            dtr: false,
            poll_interval: 0,
            timeout: 0,
        }
    }
}

pub struct RigModel {
    rig: Rig,
}

#[component(pub)]
impl SimpleComponent for RigModel {
    type Init = Rig;
    type Input = RigMessage;
    type Output = ();

    view! {
        gtk::Grid {
            set_column_homogeneous: true,
            set_row_spacing: 4,
            set_column_spacing: 4,
            set_margin_top: 8,
            set_margin_bottom: 8,
            set_margin_start: 8,
            set_margin_end: 8,

            attach[1, 1, 1, 1] = &gtk::Label {
                set_label: "Rig Type",
            },
            attach[2, 1, 1, 1] = &gtk::ComboBox {
            },
            attach[1, 2, 1, 1] = &gtk::Label {
                set_label: "Port",
            },
            attach[2, 2, 1, 1] = &gtk::ComboBox {
            },
            attach[1, 3, 1, 1] = &gtk::Label {
                set_label: "Baud rate",
            },
            attach[2, 3, 1, 1] = &gtk::ComboBox {
            },
            attach[1, 4, 1, 1] = &gtk::Label {
                set_label: "Data bits",
            },
            attach[2, 4, 1, 1] = &gtk::ComboBox {
            },
            attach[1, 5, 1, 1] = &gtk::Label {
                set_label: "Parity",
            },
            attach[2, 5, 1, 1] = &gtk::ComboBox {
            },
            attach[1, 6, 1, 1] = &gtk::Label {
                set_label: "Stop bits",
            },
            attach[2, 6, 1, 1] = &gtk::ComboBox {
            },
            attach[1, 7, 1, 1] = &gtk::Label {
                set_label: "RTS",
            },
            attach[2, 7, 1, 1] = &gtk::ComboBox {
            },
            attach[1, 8, 1, 1] = &gtk::Label {
                set_label: "DTR",
            },
            attach[2, 8, 1, 1] = &gtk::ComboBox {
            },
            attach[1, 9, 1, 1] = &gtk::Label {
                set_label: "Poll interval",
            },
            attach[2, 9, 1, 1] = &gtk::ComboBox {
            },
            attach[1, 10, 1, 1] = &gtk::Label {
                set_label: "Timeout",
            },
            attach[2, 10, 1, 1] = &gtk::ComboBox {
            },
        },
    }

    fn init(
        rig: Self::Init,
        window: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = RigModel { rig };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {}
    }
}
