use crate::rpser::xml::{BuildElement, EnhancedNode, Error as XMLError};
use xmltree::XMLNode;

use crate::server_info::RemoteServerInfo;
use crate::{AttachmentResponse, Page, PageSummary, Space};

pub trait FromXMLNode {
    fn from_node(node: XMLNode) -> Result<Self, XMLError>
    where
        Self: Sized;
}

impl FromXMLNode for Space {
    fn from_node(node: XMLNode) -> Result<Self, XMLError> {
        if let XMLNode::Element(element) = node {
            Ok(Space {
                description: element
                    .get_at_path(&["description"])?
                    .as_text()
                    .map(Into::into),
                home_page: element
                    .get_at_path(&["homePage"])
                    .and_then(|e| e.as_long())?,
                key: element.get_at_path(&["key"]).and_then(|e| e.as_string())?,
                name: element.get_at_path(&["name"]).and_then(|e| e.as_string())?,
                space_group: element.get_at_path(&["name"])?.as_text().map(Into::into),
                space_type: element.get_at_path(&["type"]).and_then(|e| e.as_string())?,
                url: element.get_at_path(&["url"]).and_then(|e| e.as_string())?,
            })
        } else {
            Err(XMLError::ExpectedElement { found: node })
        }
    }
}

impl FromXMLNode for Page {
    fn from_node(node: XMLNode) -> Result<Self, XMLError> {
        if let XMLNode::Element(element) = node {
            Ok(Page {
                id: element.get_at_path(&["id"]).and_then(|e| e.as_long())?,
                space: element
                    .get_at_path(&["space"])
                    .and_then(|e| e.as_string())?,
                parent_id: element
                    .get_at_path(&["parentId"])
                    .and_then(|e| e.as_long())?,
                title: element
                    .get_at_path(&["title"])
                    .and_then(|e| e.as_string())?,
                url: element.get_at_path(&["url"]).and_then(|e| e.as_string())?,
                version: element.get_at_path(&["version"]).and_then(|e| e.as_int())?,
                content: element
                    .get_at_path(&["content"])
                    .and_then(|e| e.as_string())?,
                created: element
                    .get_at_path(&["created"])
                    .and_then(|e| e.as_datetime())?,
                creator: element
                    .get_at_path(&["creator"])
                    .and_then(|e| e.as_string())?,
                modified: element
                    .get_at_path(&["modified"])
                    .and_then(|e| e.as_datetime())?,
                modifier: element
                    .get_at_path(&["modifier"])
                    .and_then(|e| e.as_string())?,
                home_page: element
                    .get_at_path(&["homePage"])
                    .and_then(|e| e.as_boolean())?,
                content_status: element
                    .get_at_path(&["contentStatus"])
                    .and_then(|e| e.as_string())?,
                current: element
                    .get_at_path(&["current"])
                    .and_then(|e| e.as_boolean())?,
            })
        } else {
            Err(XMLError::ExpectedElement { found: node })
        }
    }
}

impl FromXMLNode for PageSummary {
    fn from_node(node: XMLNode) -> Result<Self, XMLError> {
        if let XMLNode::Element(element) = node {
            Ok(PageSummary {
                id: element.get_at_path(&["id"]).and_then(|e| e.as_long())?,
                space: element
                    .get_at_path(&["space"])
                    .and_then(|e| e.as_string())?,
                parent_id: element
                    .get_at_path(&["parentId"])
                    .and_then(|e| e.as_long())?,
                title: element
                    .get_at_path(&["title"])
                    .and_then(|e| e.as_string())?,
                url: element.get_at_path(&["url"]).and_then(|e| e.as_string())?,
            })
        } else {
            Err(XMLError::ExpectedElement { found: node })
        }
    }
}

impl FromXMLNode for RemoteServerInfo {
    fn from_node(node: XMLNode) -> Result<Self, XMLError> {
        if let XMLNode::Element(element) = node {
            Ok(RemoteServerInfo {
                base_url: element
                    .get_at_path(&["baseUrl"])
                    .and_then(|e| e.as_string())
                    .ok(),
                build_id: element
                    .get_at_path(&["buildId"])
                    .and_then(|e| e.as_string())
                    .ok(),
                development_build: element
                    .get_at_path(&["developmentBuild"])
                    .and_then(|e| e.as_boolean())?,
                major_version: element
                    .get_at_path(&["majorVersion"])
                    .and_then(|e| e.as_int())?,
                minor_version: element
                    .get_at_path(&["minorVersion"])
                    .and_then(|e| e.as_int())?,
                patch_level: element
                    .get_at_path(&["patchLevel"])
                    .and_then(|e| e.as_int())?,
            })
        } else {
            Err(XMLError::ExpectedElement { found: node })
        }
    }
}

impl FromXMLNode for AttachmentResponse {
    fn from_node(node: XMLNode) -> Result<Self, XMLError> {
        if let XMLNode::Element(element) = node {
            Ok(AttachmentResponse {
                comment: element
                    .get_at_path(&["comment"])
                    .and_then(|e| e.as_string())
                    .ok(),
                content_type: element
                    .get_at_path(&["contentType"])
                    .and_then(|e| e.as_string())
                    .ok(),
                created: element
                    .get_at_path(&["created"])
                    .and_then(|e| e.as_datetime())
                    .ok(),
                creator: element
                    .get_at_path(&["creator"])
                    .and_then(|e| e.as_string())
                    .ok(),
                file_name: element
                    .get_at_path(&["fileName"])
                    .and_then(|e| e.as_string())
                    .ok(),
                file_size: element
                    .get_at_path(&["fileSize"])
                    .and_then(|e| e.as_long())?,
                id: element.get_at_path(&["id"]).and_then(|e| e.as_long())?,
                page_id: element.get_at_path(&["pageId"]).and_then(|e| e.as_long())?,
                title: element
                    .get_at_path(&["title"])
                    .and_then(|e| e.as_string())
                    .ok(),
                url: element
                    .get_at_path(&["url"])
                    .and_then(|e| e.as_string())
                    .ok(),
            })
        } else {
            Err(XMLError::ExpectedElement { found: node })
        }
    }
}
