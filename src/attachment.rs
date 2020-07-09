use crate::rpser::xml::BuildElement;
use chrono::{DateTime, Utc};
use mime_guess::Mime;
use xmltree::Element;

pub struct AttachmentRequest {
    file_name: String,
    content_type: Mime,
    title: Option<String>,
    comment: Option<String>,
}

pub struct AttachmentResponse {
    pub comment: Option<String>,
    pub content_type: Option<String>,
    pub created: Option<DateTime<Utc>>,
    pub creator: Option<String>,
    pub file_name: Option<String>,
    pub file_size: i64,
    pub id: i64,
    pub page_id: i64,
    pub title: Option<String>,
    pub url: Option<String>,
}

impl AttachmentRequest {
    pub fn new<N, T, C>(file_name: N, content_type: Mime, title: T, comment: C) -> AttachmentRequest
    where
        N: Into<String>,
        T: Into<Option<String>>,
        C: Into<Option<String>>,
    {
        AttachmentRequest {
            file_name: file_name.into(),
            content_type,
            title: title.into(),
            comment: comment.into(),
        }
    }
}

impl Into<Element> for AttachmentRequest {
    fn into(self) -> Element {
        let mut children = vec![];

        children.push(Element::node("fileName").with_text(self.file_name));
        children.push(Element::node("contentType").with_text(format!("{}", self.content_type)));
        if let Some(title) = self.title {
            children.push(Element::node("title").with_text(title));
        }
        if let Some(comment) = self.comment {
            children.push(Element::node("comment").with_text(comment));
        }

        Element::node("attachment").with_children(children)
    }
}
