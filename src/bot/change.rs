use std::fmt::{Display, Formatter};
use crate::tencent::model::ApplicationProgress;

#[derive(Clone)]
pub enum StatusChange {
    Progress(ApplicationProgress),
    Expiry
}

impl Display for StatusChange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusChange::Progress(ap) => {
                match ap.get_current_step() {
                    Ok(Some(step)) => write!(f, "{:?}", step),
                    Ok(None) => write!(f, "empty"),
                    Err(e) => write!(f, "{} error", e)
                }
            }
            StatusChange::Expiry => write!(f, "token expiry")
        }
    }
}