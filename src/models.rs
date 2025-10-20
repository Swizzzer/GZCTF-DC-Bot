use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Notice {
    pub id: u64,
    #[serde(rename = "type")]
    pub notice_type: String,
    pub values: Vec<String>,
    pub time: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NoticeType {
    Normal,
    NewChallenge,
    NewHint,
    FirstBlood,
    SecondBlood,
    ThirdBlood,
}

impl NoticeType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Normal" => Some(NoticeType::Normal),
            "NewChallenge" => Some(NoticeType::NewChallenge),
            "NewHint" => Some(NoticeType::NewHint),
            "FirstBlood" => Some(NoticeType::FirstBlood),
            "SecondBlood" => Some(NoticeType::SecondBlood),
            "ThirdBlood" => Some(NoticeType::ThirdBlood),
            _ => None,
        }
    }

    pub fn get_title(&self) -> &str {
        match self {
            NoticeType::Normal => "【比赛公告】",
            NoticeType::NewChallenge => "【新增题目】",
            NoticeType::NewHint => "【题目提示】",
            NoticeType::FirstBlood => "【一血播报】",
            NoticeType::SecondBlood => "【二血播报】",
            NoticeType::ThirdBlood => "【三血播报】",
        }
    }

    pub fn all() -> Vec<NoticeType> {
        vec![
            NoticeType::Normal,
            NoticeType::NewChallenge,
            NoticeType::NewHint,
            NoticeType::FirstBlood,
            NoticeType::SecondBlood,
            NoticeType::ThirdBlood,
        ]
    }
}
