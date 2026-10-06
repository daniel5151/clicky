use crate::gui::{IpodBinds, IpodKey};
use crate::signal::gpio;

/// The GPIO lines driven by the 1st gen iPod mini's controls.
/// Clickwheel also communicates over GPIO on this model.
#[derive(Debug)]
pub(super) struct IpodMini1gControls {
    pub hold: gpio::Sender,
    pub action: gpio::Sender,
    pub up: gpio::Sender,
    pub down: gpio::Sender,
    pub left: gpio::Sender,
    pub right: gpio::Sender,
}

impl IpodMini1gControls {
    /// Wrap the control lines in a set of frontend-agnostic callbacks.
    pub fn into_binds(self) -> IpodBinds {
        let IpodMini1gControls {
            mut hold,
            mut action,
            mut up,
            mut down,
            mut left,
            mut right,
        } = self;

        let mut controls = IpodBinds::default();

        controls.keys.insert(
            IpodKey::Hold,
            Box::new(move |pressed| {
                if pressed {
                    // toggle on and off
                    match hold.is_set_high() {
                        false => hold.set_high(),
                        true => hold.set_low(),
                    }
                }
            }),
        );

        macro_rules! connect_controls_btn {
            ($key:expr, $signal:expr) => {
                controls.keys.insert(
                    $key,
                    Box::new(move |pressed| {
                        // buttons are active-low
                        if pressed {
                            $signal.set_low()
                        } else {
                            $signal.set_high()
                        }
                    }),
                );
            };
        }

        connect_controls_btn!(IpodKey::Up, up);
        connect_controls_btn!(IpodKey::Down, down);
        connect_controls_btn!(IpodKey::Left, left);
        connect_controls_btn!(IpodKey::Right, right);
        connect_controls_btn!(IpodKey::Action, action);

        // TODO: the wheel is a quadrature encoder on GPIO B4 / B5

        controls
    }
}
