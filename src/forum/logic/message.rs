use dapp_lib::{prelude::ContentID, AppDefinedMsg, Data};

use crate::forum::logic::Entry;

#[derive(Debug, Clone, PartialEq)]
pub enum ForumSyncMessage {
    EditPost(ContentID, u16, Entry),
    AddTopic(ContentID, Entry),
    AddPost(ContentID, Entry),
}
impl ForumSyncMessage {
    pub fn parse_app_msg(app_msg: AppDefinedMsg) -> Option<Self> {
        match app_msg.m_type() {
            0 => {
                let d_id = app_msg.d_id();

                Some(ForumSyncMessage::EditPost(
                    app_msg.c_id(),
                    d_id,
                    Entry::from_data(app_msg.data(), d_id > 0).unwrap(),
                ))
            }
            1 => Some(ForumSyncMessage::AddTopic(
                app_msg.c_id(),
                Entry::from_data(app_msg.data(), false).unwrap(),
            )),
            2 => Some(ForumSyncMessage::AddPost(
                app_msg.c_id(),
                Entry::from_data(app_msg.data(), true).unwrap(),
            )),
            _o => None,
        }
    }

    pub fn into_app_msg(self) -> Result<AppDefinedMsg, (u8, ContentID, u16, Data)> {
        match self {
            Self::EditPost(c_id, d_id, entry) => {
                // AppDefinedMsg::new(0, c_id, d_id, entry.into_data(d_id > 0).unwrap())
                AppDefinedMsg::new(0, c_id, d_id, entry.into_data().unwrap())
            }
            Self::AddTopic(c_id, entry) => {
                // AppDefinedMsg::new(1, c_id, 0, entry.into_data(false).unwrap())
                AppDefinedMsg::new(1, c_id, 0, entry.into_data().unwrap())
            }
            Self::AddPost(c_id, entry) => {
                // AppDefinedMsg::new(2, c_id, 0, entry.into_data(true).unwrap())
                AppDefinedMsg::new(2, c_id, 0, entry.into_data().unwrap())
            }
        }
    }
}
