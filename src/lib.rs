/*!
Access and modify [Atlassian Confluence](https://www.atlassian.com/software/confluence/) pages from Rust.

## Working with this library

To start, create a new `Session` by calling a `login` on it
with your credentials.

Internally, the `Session` struct stores the auth `token`
and uses it when calling remote methods.

The token will be destroyed (automatic logout) when `Session` goes out of scope.
*/

#[macro_use]
extern crate log;
extern crate chrono;
extern crate reqwest;
extern crate xml;
extern crate xmltree;

pub mod http;
pub mod rpser;
pub mod wsdl;

mod attachment;
mod page;
mod server_info;
mod space;
mod transforms;

pub use attachment::{AttachmentRequest, AttachmentResponse};
pub use page::{Page, PageSummary, PageUpdateOptions, UpdatePage};
pub use server_info::RemoteServerInfo;
pub use space::Space;
pub use transforms::FromXMLNode;

use std::io::{Error as IoError, Read};
use std::{io, result};

use self::http::HttpError;
use self::rpser::xml::{BuildElement, EnhancedNode};
use self::rpser::{Method, RpcError};
use core::mem;
use mime_guess::MimeGuess;
use reqwest::Client;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::File;
use std::path::PathBuf;
use xmltree::Element;

const V2_API_RPC_PATH: &str = "/rpc/soap-axis/confluenceservice-v2?wsdl";

/// Client's session.
pub struct Session {
    wsdl: wsdl::Wsdl,
    token: String,
    client: Client,
}

impl Drop for Session {
    fn drop(&mut self) {
        futures::executor::block_on(self.internal_logout()).unwrap();
    }
}

impl Session {
    /**
    Create new confluence session.

    ## Example

    ```no_run
    # async {
    let session = confluence::Session::login(
        "https://confluence",
        "user",
        "pass"
    ).await.unwrap();
    # };
    ```
    */
    pub async fn login(url: &str, user: &str, pass: &str) -> Result<Session> {
        debug!("logging in at url {:?} with user {:?}", url, user);

        let client = Client::new();

        let url = if url.ends_with('/') {
            &url[..url.len() - 1]
        } else {
            url
        };
        let wsdl_url = [url, V2_API_RPC_PATH].concat();

        debug!("getting wsdl from url {:?}", wsdl_url);

        let wsdl = wsdl::fetch(&wsdl_url).await?;

        let response = Session::internal_call(
            Method::new("login")
                .with(Element::node("username").with_text(user))
                .with(Element::node("password").with_text(pass)),
            &wsdl,
            &client,
        )
        .await?;

        let token = match response
            .body
            .descend(&["loginReturn"])?
            .expect_element()?
            .get_text()
            .map(Cow::into_owned)
        {
            Some(token) => token,
            _ => return Err(Error::ReceivedNoLoginToken),
        };

        Ok(Session {
            wsdl,
            token,
            client,
        })
    }

    /// Explicitly log out out of confluence.
    ///
    /// This is done automatically at the end of Session's lifetime.
    pub async fn logout(mut self) -> Result<bool> {
        // We want to know the result of logging out. However we also want to logout when dropping
        // our memory AND we want to make Self::logout a dropping operation. To do all of these
        // things we have to leverage the private method internal_logout which checks whether we've
        // already logged out before running
        self.internal_logout().await
    }

    async fn internal_logout(&mut self) -> Result<bool> {
        if !self.token.is_empty() {
            let response = self.call(self.method("logout")).await?;

            match response
                .body
                .descend(&["logoutReturn"])?
                .expect_element()?
                .get_text()
            {
                Some(ref v) if v == "true" => {
                    debug!("logged out successfully");
                    Ok(true)
                }
                _ => {
                    debug!("log out failed (maybe expired token, maybe not logged in)");
                    Ok(false)
                }
            }
        } else {
            // we've already logged out so continue on
            Ok(false)
        }
    }

    /**
    Returns a single Space.

    If the spaceKey does not exist: earlier versions of Confluence will throw an Exception. Later versions (3.0+) will return a null object.

    In this client the difference will be in error type.

    ## Example

    ```no_run
    # async {
    # let session = confluence::Session::login("https://confluence", "user", "pass").await.unwrap();
    println!("Space: {:#?}",
        session.get_space(
            "SomeSpaceKey"
        ).await
    );
    # };
    ```
    */
    pub async fn get_space(&self, space_key: &str) -> Result<Space> {
        let response = self
            .call(
                self.method("getSpace")
                    .with(Element::node("spaceKey").with_text(space_key)),
            )
            .await?;

        let element = response.body.descend(&["getSpaceReturn"])?;

        Space::from_node(element).map_err(Into::into)
    }

    /**
    Returns a single Page by space and title.

    ## Example

    ```no_run
    # async {
    # let session = confluence::Session::login("https://confluence", "user", "pass").await.unwrap();
    println!("Page: {:#?}",
        session.get_page_by_title(
            "SomeSpaceKey", "Page Title"
        ).await
    );
    # };
    ```
    */
    pub async fn get_page_by_title(&self, space_key: &str, page_title: &str) -> Result<Page> {
        let response = self
            .call(
                self.method("getPage")
                    .with(Element::node("spaceKey").with_text(space_key))
                    .with(Element::node("pageTitle").with_text(page_title)),
            )
            .await?;

        let element = response.body.descend(&["getPageReturn"])?;

        Page::from_node(element).map_err(Into::into)
    }

    /**
    Returns a single Page by id.

    ## Example

    ```no_run
    # async {
    # let session = confluence::Session::login("https://confluence", "user", "pass").await.unwrap();
    println!("Page: {:#?}",
        session.get_page_by_id(
            123456
        ).await
    );
    # };
    ```
    */
    pub async fn get_page_by_id(&self, page_id: i64) -> Result<Page> {
        let response = self
            .call(
                self.method("getPage")
                    .with(Element::node("pageId").with_text(page_id.to_string())),
            )
            .await?;

        let element = response.body.descend(&["getPageReturn"])?;

        Page::from_node(element).map_err(Into::into)
    }

    /**
    Adds or updates a page.

    # For adding

    The Page given as an argument should have:

    - (optional) parent_id
    - space
    - title
    - content

    fields at a minimum.

    Use helper `UpdatePage::with_create_fields` to create such page.

    ## Example

    ```no_run
    use confluence::UpdatePage;

    # async {
    # let session = confluence::Session::login("https://confluence", "user", "pass").await.unwrap();
    session.store_page(
        UpdatePage::with_create_fields(
            None,
            "SpaceKey",
            "Page Title",
            "<b>Works</b>"
        )
    ).await;
    # };
    ```

    # For updating

    The Page given should have:

    - (optional) parent_id
    - id
    - space
    - title
    - content
    - version

    fields at a minimum.

    Use method `into` on `Page` to convert it to `UpdatePage`.

    ## Example

    ```no_run
    use confluence::UpdatePage;
    # async {
    # let session = confluence::Session::login("https://confluence", "user", "pass").await.unwrap();
    let mut page = session.get_page_by_title(
        "SomeSpaceKey", "Page Title"
    ).await.unwrap();

    page.title = "New Page Title".into();

    session.store_page(page.into()).await;
    # };
    ```
    */
    pub async fn store_page(&self, page: UpdatePage) -> Result<Page> {
        let mut element_items = vec![
            Element::node("space").with_text(page.space),
            Element::node("title").with_text(page.title),
            Element::node("content").with_text(page.content),
        ];

        if let Some(id) = page.id {
            element_items.push(Element::node("id").with_text(id.to_string()));
        }

        if let Some(version) = page.version {
            element_items.push(Element::node("version").with_text(version.to_string()));
        }

        if let Some(parent_id) = page.parent_id {
            element_items.push(Element::node("parentId").with_text(parent_id.to_string()));
        }

        let response = self
            .call(
                self.method("storePage")
                    .with(Element::node("page").with_children(element_items)),
            )
            .await?;

        let element = response.body.descend(&["storePageReturn"])?;

        Page::from_node(element).map_err(Into::into)
    }

    /**
    Updates the page.

    Same as `store_page`, but with additional update options parameter.
    */
    pub async fn update_page(&self, page: UpdatePage, options: PageUpdateOptions) -> Result<Page> {
        let mut element_items = vec![
            Element::node("space").with_text(page.space),
            Element::node("title").with_text(page.title),
            Element::node("content").with_text(page.content),
        ];

        if let Some(id) = page.id {
            element_items.push(Element::node("id").with_text(id.to_string()));
        }

        if let Some(version) = page.version {
            element_items.push(Element::node("version").with_text(version.to_string()));
        }

        if let Some(parent_id) = page.parent_id {
            element_items.push(Element::node("parentId").with_text(parent_id.to_string()));
        }

        let mut update_options = vec![];

        if let Some(comment) = options.version_comment {
            update_options.push(Element::node("versionComment").with_text(comment));
        }

        update_options.push(Element::node("minorEdit").with_text(if options.minor_edit {
            "true"
        } else {
            "false"
        }));

        let response = self
            .call(
                self.method("updatePage")
                    .with(Element::node("page").with_children(element_items))
                    .with(Element::node("pageUpdateOptions").with_children(update_options)),
            )
            .await?;

        let element = response.body.descend(&["updatePageReturn"])?;

        Page::from_node(element).map_err(Into::into)
    }

    /// Deletes a page, returning true if the page was successfully deleted
    ///
    /// ```no_run
    /// # async {
    /// # let session = confluence::Session::login("https://confluence", "user", "pass").await.unwrap();
    ///
    /// session.remove_page(123456).await;
    ///
    /// # };
    /// ```
    pub async fn remove_page(&self, page_id: i64) -> Result<bool> {
        let response = self
            .call(
                self.method("removePage")
                    .with(Element::node("pageId").with_text(page_id.to_string())),
            )
            .await?;

        match response
            .body
            .descend(&["removePageReturn"])?
            .expect_element()?
            .get_text()
        {
            Some(ref v) if v == "true" => Ok(true),
            _ => Ok(false),
        }
    }

    /// Adds a file as an Attachment
    ///
    /// You can either pass in your own `AttachmentRequest` or if you leave it out, the relevant
    /// info will attempt to be derived from your `PathBuf`
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use futures::future::TryFutureExt;
    ///
    /// # async {
    /// # let session = confluence::Session::login("https://confluence", "user", "pass").await.unwrap();
    ///
    /// let path_buf = PathBuf::from("image.png");
    /// session.add_file(1234, None, &path_buf);
    /// # };
    /// ```
    ///
    /// will derive:
    /// * `file_name`: `"image.png"`
    /// * `content_type`: `mime::IMAGE_PNG`
    /// * `title`: `None`
    /// * `comment`: `None`
    pub async fn add_file<A>(
        &self,
        content_id: i64,
        attachment: A,
        file_path: &PathBuf,
    ) -> Result<AttachmentResponse>
    where
        A: Into<Option<AttachmentRequest>>,
    {
        let file = File::open(file_path)?;
        let attachment_request = attachment.into().unwrap_or_else(|| {
            AttachmentRequest::new(
                file_path.file_name().and_then(OsStr::to_str).unwrap_or(""),
                MimeGuess::from_path(&file_path).first_or_octet_stream(),
                None,
                None,
            )
        });

        self.add_attachment(content_id, attachment_request, file)
            .await
    }

    /// Add an Attachment for the content
    ///
    /// The `content_id` can the be the Page you want to attach the file to.
    ///
    /// ```no_run
    /// # use confluence::AttachmentRequest;
    /// # use mime_guess::mime;
    /// # use std::fs::File;
    ///
    /// # async {
    /// # let session = confluence::Session::login("https://confluence", "user", "pass").await.unwrap();
    ///
    /// if let Ok(file) = File::open("file") {
    ///     session.add_attachment(
    ///         123456,
    ///         AttachmentRequest::new("File Name", mime::IMAGE_PNG, "Title".to_string(), "New image".to_string()),
    ///         &file,
    ///     );
    /// }
    ///
    /// # };
    /// ```
    pub async fn add_attachment<R: Read>(
        &self,
        content_id: i64,
        attachment: AttachmentRequest,
        mut data_reader: R,
    ) -> Result<AttachmentResponse> {
        let mut written_data = vec![];
        let mut writer = base64::write::EncoderWriter::new(&mut written_data, base64::STANDARD);
        io::copy(&mut data_reader, &mut writer)?;
        mem::drop(writer);

        let data = String::from_utf8(written_data).expect("Base64 should always be valid UTF-8");

        let response = self
            .call(
                Method::new("addAttachment")
                    .with(Element::node("token").with_text(self.token()))
                    .with(Element::node("contentId").with_text(content_id.to_string()))
                    .with(attachment.into())
                    .with(Element::node("attachmentData").with_text(data)),
            )
            .await?;

        AttachmentResponse::from_node(response.body.descend(&["addAttachmentReturn"])?)
            .map_err(Into::into)
    }

    /**
    Returns all the direct children of this page.

    ## Example

    ```no_run
    # async {
    # let session = confluence::Session::login("https://confluence", "user", "pass").await.unwrap();
    println!("Page Summaries: {:#?}",
        session.get_children(
            123456
        ).await
    );
    # };
    ```
    */
    pub async fn get_children(&self, page_id: i64) -> Result<Vec<PageSummary>> {
        let response = self
            .call(
                self.method("getChildren")
                    .with(Element::node("pageId").with_text(page_id.to_string())),
            )
            .await?;

        let node = response.body.descend(&["getChildrenReturn"])?;

        let mut summaries = vec![];

        for element in node.into_element()?.children {
            summaries.push(PageSummary::from_node(element)?);
        }

        Ok(summaries)
    }

    /// Gets information about the Confluence server this session is connected to
    ///
    /// ```no_run
    /// # async {
    /// # let session = confluence::Session::login("https://confluence", "user", "pass").await.unwrap();
    ///
    /// session.get_server_info().await;
    ///
    /// # };
    /// ```
    pub async fn get_server_info(&self) -> Result<RemoteServerInfo> {
        let response = self.call(self.method("getServerInfo")).await?;

        let node = response.body.descend(&["getServerInfoReturn"])?;

        RemoteServerInfo::from_node(node).map_err(Into::into)
    }

    /// builds a new method with the token from the session already set
    pub fn method(&self, name: &str) -> Method {
        Method::new(name).with(Element::node("token").with_text(self.token.clone()))
    }

    /// Get the token for our session
    pub fn token(&self) -> String {
        self.token.clone()
    }

    /// Call a custom method on this session.
    ///
    /// ## Usage
    ///
    /// The elements in `Method` struct here will be converted directly
    /// into SOAP envelope's Body.
    ///
    /// The returned `Response`.`body` will contain the parsed Body element.
    ///
    /// ## Discussion
    ///
    /// So far only few methods have convenience wrappers here, so if you need to call [something
    /// else](https://developer.atlassian.com/confdev/confluence-rest-api/confluence-xml-rpc-and-soap-apis/remote-confluence-methods),
    /// it's not so convenient, but possible.
    ///
    /// If you need an example, look at how these convenience methods are implemented.
    ///
    /// Pull requests are welcome!
    pub async fn call(&self, method: rpser::Method) -> Result<rpser::Response> {
        Self::internal_call(method, &self.wsdl, &self.client).await
    }

    async fn internal_call(
        method: rpser::Method,
        wsdl: &wsdl::Wsdl,
        client: &Client,
    ) -> Result<rpser::Response> {
        let url = match wsdl.operations.get(&method.name) {
            None => return Err(Error::MethodNotFoundInWsdl(method.name)),
            Some(ref op) => &op.url,
        };

        // do now show password in logs
        if method.name == "login" {
            debug!("[call] login ******");
        } else {
            debug!("[call] {}", method);
        }

        let envelope = method.as_xml(url);

        // do now show password in logs
        if method.name != "login" {
            trace!("[method xml] {}", envelope);
        }

        let http_response = http::soap_action(url, &method.name, &envelope, client).await?;

        trace!("[response xml] {}", http_response.body);

        Ok(rpser::Response::from_xml(&http_response.body)?)
    }
}

/// Confluence library error.
#[derive(Debug)]
pub enum Error {
    MethodNotFoundInWsdl(String),
    ReceivedNoLoginToken,
    Io(IoError),
    Http(HttpError),
    Rpc(Box<RpcError>),
}

impl From<HttpError> for Error {
    fn from(other: HttpError) -> Error {
        Error::Http(other)
    }
}

impl From<RpcError> for Error {
    fn from(other: RpcError) -> Error {
        Error::Rpc(Box::new(other))
    }
}

impl From<rpser::xml::Error> for Error {
    fn from(other: rpser::xml::Error) -> Error {
        RpcError::from(other).into()
    }
}

impl From<IoError> for Error {
    fn from(other: IoError) -> Error {
        Error::Io(other)
    }
}

pub type Result<T> = result::Result<T, Error>;
