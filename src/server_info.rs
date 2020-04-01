/// RemoteServerInfo.
#[derive(Debug)]
pub struct RemoteServerInfo {
    pub base_url: Option<String>,

    pub build_id: Option<String>,

    pub development_build: bool,

    pub major_version: i32,

    pub minor_version: i32,

    pub patch_level: i32,
}
