use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CurrentStatus {
    pub status: u8,
    pub apply_process_type: u8,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PositionInfo {
    pub apply_position_txt: String,
    pub interview_position_txt: String,
    pub sub_direction_id_txt: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeStatus {
    pub status: u8,
    pub is_public: u8,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentInfo {
    pub status: u8,
    pub test_address: String,
    pub mobile_tail: String
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WrittenTestInfo {
    pub status: u8,
    pub item_list: Vec<ListItem>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CampusRecruitI {
    pub id: u32,
    pub item_list: Vec<ListItem>,
    pub recruit_type: u32,
    pub type_name: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CampusRecruitII {
    pub reply_token: Option<String>,
    pub item_list: Vec<ListItem>,
    pub bgid: u32,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    pub step_id: u32,
    pub status: u8,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationProgress {
    pub resume_id: u32,
    pub current_status: CurrentStatus,
    pub assessment_info: AssessmentInfo,
    pub position_info: PositionInfo,
    pub resume_status: ResumeStatus,
    pub written_test_info: WrittenTestInfo,
    pub campus_recruit_one: CampusRecruitI,
    pub campus_recruit_two: CampusRecruitII,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetApplyProcessResponse {
    pub message: String,
    pub status: u16,
    pub data: ApplicationProgress
}