use crate::tencent::model::ApplicationProgress;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Step {
    CvDeliverance,
    Examination,
    WrittenTest,
    GroupInterview,
    PreliminaryInterview,
    SecondaryInterview,
    HrInterview,
    EmployerAssessment,
    EmployeeConfirmation,
    OfferConfirmation,
    SignUp,
    Completed,
}

impl ApplicationProgress {
    pub fn get_current_step(&self) -> Result<Option<Step>, Error> {
        if self.resume_status.status < 3 {
            return if self.resume_status.status < 2 {
                Ok(None)
            } else {
                Ok(Some(Step::CvDeliverance))
            };
        }

        if self.assessment_info.status == 2 {
            return Ok(Some(Step::Examination));
        }

        if self.written_test_info.status == 2 {
            return Ok(Some(Step::WrittenTest));
        }

        let mut r1_list = self.campus_recruit_one.item_list.clone();
        r1_list.sort_by(|a, b| b.step_id.cmp(&a.step_id));

        let r1_current = r1_list.iter().find(|item| item.status == 2);
        if let Some(item) = r1_current {
            return recruit_one_step(item.step_id).map(|s| Some(s));
        }

        if let (Some(first_step), Some(last_step)) = (r1_list.last(), r1_list.first()) {
            if first_step.status == 1 && last_step.status < 3 {
                return Ok(None);
            }
        }

        let mut r2_list = self.campus_recruit_two.item_list.clone();
        r2_list.sort_by(|a, b| b.step_id.cmp(&a.step_id));

        let r2_current = r2_list.iter().find(|item| item.status == 2);
        if let Some(item) = r2_current {
            return recruit_two_step(item.step_id).map(|s| Some(s));
        }

        Ok(Some(Step::Completed)) // TODO: I haven't landed here yet
    }
}

fn recruit_one_step(step_id: u32) -> Result<Step, Error> {
    Ok(match step_id {
        1 => Step::GroupInterview,
        2 => Step::PreliminaryInterview,
        3 => Step::SecondaryInterview,
        5 => Step::HrInterview,
        _ => return Err(Error::UnknownStep(step_id)),
    })
}

fn recruit_two_step(step_id: u32) -> Result<Step, Error> {
    Ok(match step_id {
        1 => Step::EmployerAssessment,
        2 => Step::EmployeeConfirmation,
        3 => Step::OfferConfirmation,
        _ => return Err(Error::UnknownStep(step_id)),
    })
}

pub enum Error {
    UnknownStep(u32),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownStep(id) => write!(f, "unknown step {}", id),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tencent::Client;
    use std::env;

    #[tokio::test]
    async fn fetches_ap() {
        let client = Client::with_token(
            &env::var("USER_INFO").expect("Environment variable USER_INFO is missing"),
        );
        client.get_application_progress().await.expect("Error fetching");
    }
}
