use super::{LiveControl, ViewerControl};

impl TryFrom<ViewerControl> for LiveControl {
    type Error = &'static str;
    fn try_from(value: ViewerControl) -> Result<Self, Self::Error> {
        match value {
            ViewerControl::Pause => Ok(Self::Pause),
            ViewerControl::Play => Ok(Self::Play),
            ViewerControl::Step { count } => Ok(Self::Step { count }),
            ViewerControl::Seek { .. } => Err("seek is not valid in live control mode"),
        }
    }
}

impl From<LiveControl> for ViewerControl {
    fn from(value: LiveControl) -> Self {
        match value {
            LiveControl::Pause => Self::Pause,
            LiveControl::Play => Self::Play,
            LiveControl::Step { count } => Self::Step { count },
        }
    }
}
