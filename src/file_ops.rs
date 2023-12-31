use std::fmt::Display;

use crate::{
    folder_ops::FolderDescriptor,
    pcloud_client::PCloudClient,
    pcloud_model::{
        self, FileOrFolderStat, Metadata, PCloudResult, PublicFileLink, RevisionList, UploadedFile,
        WithPCloudResult,
    },
};
use chrono::{DateTime, TimeZone};
use log::debug;
use reqwest::{Body, RequestBuilder, Response};

/// Generic description of a pCloud File. Either by its file id (preferred) or by its path. Optionally give tuple with id / path and file revision
pub trait FileDescriptor {
    /// Convert the descriptor into a PCloudFile
    fn to_file(self) -> Result<PCloudFile, PCloudResult>;
}

impl FileDescriptor for u64 {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(self.into())
    }
}

impl FileDescriptor for String {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(self.into())
    }
}

impl FileDescriptor for &str {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(self.into())
    }
}

impl FileDescriptor for &u64 {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(self.into())
    }
}

impl FileDescriptor for &Metadata {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        self.try_into()
    }
}

impl FileDescriptor for &FileOrFolderStat {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        self.try_into()
    }
}

impl FileDescriptor for PCloudFile {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(self)
    }
}

impl FileDescriptor for &PCloudFile {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(self.clone())
    }
}

/// A file path and a revision id
impl FileDescriptor for (String, u64) {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(PCloudFile {
            revision: Some(self.1),
            ..self.0.into()
        })
    }
}

/// A file path and a revision id
impl FileDescriptor for (&str, u64) {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(PCloudFile {
            revision: Some(self.1),
            ..self.0.into()
        })
    }
}

/// A file id and a revision id
impl FileDescriptor for (u64, u64) {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(PCloudFile {
            revision: Some(self.1),
            ..self.0.into()
        })
    }
}

/// A file id and a revision id
impl FileDescriptor for (&Metadata, u64) {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(PCloudFile {
            revision: Some(self.1),
            ..self.0.try_into()?
        })
    }
}

impl FileDescriptor for (&FileOrFolderStat, u64) {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(PCloudFile {
            revision: Some(self.1),
            ..self.0.try_into()?
        })
    }
}

impl FileDescriptor for (&PCloudFile, u64) {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(PCloudFile {
            revision: Some(self.1),
            ..self.0.clone()
        })
    }
}

impl FileDescriptor for (PCloudFile, u64) {
    fn to_file(self) -> Result<PCloudFile, PCloudResult> {
        Ok(PCloudFile {
            revision: Some(self.1),
            ..self.0
        })
    }
}

#[derive(Debug, Clone)]
pub struct PCloudFile {
    /// ID of the target file
    pub(crate) file_id: Option<u64>,
    /// Path of the target file
    pub(crate) path: Option<String>,
    /// File revision
    pub(crate) revision: Option<u64>,
}

impl PCloudFile {
    pub fn is_empty(&self) -> bool {
        self.file_id.is_none() && self.path.is_none()
    }
}

impl Display for PCloudFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(file_id) = self.file_id {
            if let Some(revision) = self.revision {
                write!(f, "{}@{}", file_id, revision)
            } else {
                write!(f, "{}", file_id)
            }
        } else if let Some(path) = &self.path {
            if let Some(revision) = self.revision {
                write!(f, "{}@{}", path, revision)
            } else {
                write!(f, "{}", path)
            }
        } else {
            write!(f, "[Empty pCloud file descriptor!]")
        }
    }
}

/// Convert Strings into pCloud file paths
impl From<&str> for PCloudFile {
    fn from(value: &str) -> PCloudFile {
        PCloudFile {
            file_id: None,
            path: Some(value.to_string()),
            revision: None,
        }
    }
}

/// Convert Strings into pCloud file paths
impl From<String> for PCloudFile {
    fn from(value: String) -> PCloudFile {
        PCloudFile {
            file_id: None,
            path: Some(value),
            revision: None,
        }
    }
}

/// Convert u64 into pCloud file ids
impl From<u64> for PCloudFile {
    fn from(value: u64) -> PCloudFile {
        PCloudFile {
            file_id: Some(value),
            path: None,
            revision: None,
        }
    }
}

/// Convert u64 into pCloud file ids
impl From<&u64> for PCloudFile {
    fn from(value: &u64) -> PCloudFile {
        PCloudFile {
            file_id: Some(value.clone()),
            path: None,
            revision: None,
        }
    }
}

/// Extract file id from pCloud file or folder metadata response
impl TryFrom<&Metadata> for PCloudFile {
    type Error = PCloudResult;

    fn try_from(value: &Metadata) -> Result<PCloudFile, PCloudResult> {
        if value.isfolder {
            Err(PCloudResult::InvalidFileOrFolderName)?
        } else {
            Ok(PCloudFile {
                file_id: value.fileid,
                path: None,
                revision: None,
            })
        }
    }
}

/// Extract file id from pCloud file or folder metadata response
impl TryFrom<&FileOrFolderStat> for PCloudFile {
    type Error = PCloudResult;
    fn try_from(value: &FileOrFolderStat) -> Result<PCloudFile, PCloudResult> {
        if value.result == PCloudResult::Ok && value.metadata.is_some() {
            let metadata = value.metadata.as_ref().unwrap();
            metadata.try_into()
        } else {
            Err(PCloudResult::InvalidFileOrFolderName)?
        }
    }
}

/// Some methods can work with trees - that is set of files and folders, where folders can have files and subfolders inside them and so on.
/// see https://docs.pcloud.com/structures/tree.html
pub struct Tree {
    /// Client to perform requests
    client: PCloudClient,
    /// If set, contents of the folder with the given id will appear as root elements of the three. The folder itself does not appear as a part of the structure.
    folder_id: Option<u64>,
    /// If set, defines one or more folders that will appear as folders in the root folder. If multiple folderids are given, they MUST be separated by coma ,.
    folder_ids: Vec<u64>,
    /// If set, files with corresponding ids will appear in the root folder of the tree structure. If more than one fileid is provided, they MUST be separated by coma ,.
    file_ids: Vec<u64>,
    /// If set, files with corresponding ids will appear in the root folder of the tree structure. If more than one fileid is provided, they MUST be separated by coma ,.
    exclude_folder_ids: Vec<u64>,
    /// If set, defines fileids that are not to be included in the tree structure.
    exclude_file_ids: Vec<u64>,
}

/// Some methods can work with trees - that is set of files and folders, where folders can have files and subfolders inside them and so on.
#[allow(dead_code)]
impl Tree {
    pub(crate) fn create(client: &PCloudClient) -> Tree {
        Tree {
            folder_id: None,
            folder_ids: Vec::default(),
            file_ids: Vec::default(),
            exclude_folder_ids: Vec::default(),
            exclude_file_ids: Vec::default(),
            client: client.clone(),
        }
    }

    /// Adds this tree to a request
    pub(crate) fn add_to_request(&self, mut r: RequestBuilder) -> RequestBuilder {
        if let Some(v) = self.folder_id {
            r = r.query(&[("folderid", v)]);
        }

        if !self.folder_ids.is_empty() {
            let v = self
                .folder_ids
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join(",");
            r = r.query(&[("folderids", v)]);
        }

        if !self.file_ids.is_empty() {
            let v = self
                .file_ids
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join(",");
            r = r.query(&[("fileids", v)]);
        }

        if !self.exclude_folder_ids.is_empty() {
            let v = self
                .exclude_folder_ids
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join(",");
            r = r.query(&[("excludefolderids", v)]);
        }

        if !self.exclude_file_ids.is_empty() {
            let v = self
                .exclude_file_ids
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join(",");
            r = r.query(&[("excludefileids", v)]);
        }

        return r;
    }

    /// Adds a file or folder from a metadata object
    pub async fn with(
        self,
        source: &Metadata,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if source.isfolder {
            self.with_folder(source).await
        } else {
            self.with_file(source).await
        }
    }

    /// Excludes a file or folder
    pub async fn without(
        self,
        source: &Metadata,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if source.isfolder {
            self.without_folder(source).await
        } else {
            self.without_file(source).await
        }
    }

    /// If set, files with corresponding ids will appear in the root folder of the tree structure.
    pub async fn with_file<'a, T: FileDescriptor>(
        mut self,
        file_like: T,
    ) -> Result<Self, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let (file_id, _) = self.client.get_file_id(file_like).await?;
        self.file_ids.push(file_id);
        Ok(self)
    }

    /// If set, defines fileids that are not to be included in the tree structure.
    pub async fn without_file<'a, T: FileDescriptor>(
        mut self,
        file_like: T,
    ) -> Result<Self, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let (file_id, _) = self.client.get_file_id(file_like).await?;

        self.exclude_file_ids.push(file_id);
        Ok(self)
    }

    /// If set, defines one or more folders that will appear as folders in the root folder.
    pub async fn with_folder<'a, T: FolderDescriptor>(
        mut self,
        folder_like: T,
    ) -> Result<Self, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let folder_id = self.client.get_folder_id(folder_like).await?;

        self.folder_ids.push(folder_id);
        Ok(self)
    }

    /// If set, folders with the given id will be removed from the tree structure. This is useful when you want to include a folder in the tree structure with some of it's subfolders excluded.
    pub async fn without_folder<'a, T: FolderDescriptor>(
        mut self,
        folder_like: T,
    ) -> Result<Self, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let folder_id = self.client.get_folder_id(folder_like).await?;

        self.exclude_folder_ids.push(folder_id);
        Ok(self)
    }

    /// If set, contents of the folder with the given id will appear as root elements of the tree. The folder itself does not appear as a part of the structure.
    pub async fn with_content_of_folder<'a, T: FolderDescriptor>(
        mut self,
        folder_like: T,
    ) -> Result<Self, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let folder_id = self.client.get_folder_id(folder_like).await?;

        self.folder_id = Some(folder_id);
        Ok(self)
    }
}

pub struct CopyFileRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// source file path
    from_path: Option<String>,
    /// source file id
    from_file_id: Option<u64>,
    /// destination folder path
    to_path: Option<String>,
    /// destination folder id
    to_folder_id: Option<u64>,
    /// New file name
    to_name: Option<String>,
    /// Overwrite file
    overwrite: bool,
    /// if set, file modified time is set. Have to be unix time seconds.
    mtime: Option<i64>,
    /// if set, file created time is set. It's required to provide mtime to set ctime. Have to be unix time seconds.
    ctime: Option<i64>,
    /// File revision to fetch
    revision_id: Option<u64>,
}

#[allow(dead_code)]
impl CopyFileRequestBuilder {
    pub(crate) fn copy_file<'a, S: FileDescriptor, T: FolderDescriptor>(
        client: &PCloudClient,
        file_like: S,
        target_folder_like: T,
    ) -> Result<CopyFileRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let source = file_like.to_file()?;
        let target = target_folder_like.to_folder()?;

        if !source.is_empty() && !target.is_empty() {
            Ok(CopyFileRequestBuilder {
                from_path: source.path,
                from_file_id: source.file_id,
                to_path: target.path,
                to_folder_id: target.folder_id,
                client: client.clone(),
                to_name: None,
                overwrite: true,
                mtime: None,
                ctime: None,
                revision_id: source.revision,
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    // If it is set (default true) and file with the specified name already exists, it will be overwritten
    pub fn overwrite(mut self, value: bool) -> CopyFileRequestBuilder {
        self.overwrite = value;
        self
    }

    /// if set, file modified time is set. Have to be unix time seconds.
    pub fn mtime<Tz>(mut self, value: &DateTime<Tz>) -> CopyFileRequestBuilder
    where
        Tz: TimeZone,
        Tz::Offset: Display,
    {
        self.mtime = Some(value.timestamp());
        self
    }

    ///  if set, file created time is set. It's required to provide mtime to set ctime. Have to be unix time seconds.
    pub fn ctime<Tz>(mut self, value: &DateTime<Tz>) -> CopyFileRequestBuilder
    where
        Tz: TimeZone,
        Tz::Offset: Display,
    {
        self.ctime = Some(value.timestamp());
        self
    }

    /// name of the destination file. If omitted, then the original filename is used
    pub fn with_new_name(mut self, value: &str) -> CopyFileRequestBuilder {
        self.to_name = Some(value.to_string());
        self
    }

    /// Choose the revision of the file. If not set the latest revision is used.
    pub fn with_revision(mut self, value: u64) -> CopyFileRequestBuilder {
        self.revision_id = Some(value);
        self
    }

    // Execute the copy operation
    pub async fn execute(
        self,
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error + Send + Sync>> {
        let mut r = self
            .client
            .client
            .post(format!("{}/copyfile", self.client.api_host));

        if let Some(v) = self.from_path {
            r = r.query(&[("path", v)]);
        }

        if let Some(v) = self.from_file_id {
            r = r.query(&[("fileid", v)]);
        }

        if let Some(v) = self.to_path {
            r = r.query(&[("topath", v)]);
        }

        if let Some(v) = self.to_folder_id {
            r = r.query(&[("tofolderid", v)]);
        }

        if let Some(v) = self.mtime {
            r = r.query(&[("mtime", v)]);
        }

        if let Some(v) = self.ctime {
            r = r.query(&[("ctime", v)]);
        }

        if let Some(v) = self.to_name {
            r = r.query(&[("toname", v)]);
        }

        if let Some(v) = self.revision_id {
            r = r.query(&[("revisionid", v)]);
        }

        if !self.overwrite {
            r = r.query(&[("noover", "1")]);
        }

        r = self.client.add_token(r);

        let result = r
            .send()
            .await?
            .json::<pcloud_model::FileOrFolderStat>()
            .await?
            .assert_ok()?;
        Ok(result)
    }
}

pub struct MoveFileRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// source file path
    from_path: Option<String>,
    /// source file id
    from_file_id: Option<u64>,
    /// destination folder path
    to_path: Option<String>,
    /// destination folder id
    to_folder_id: Option<u64>,
    /// New file name
    to_name: Option<String>,
    /// File revision to fetch
    revision_id: Option<u64>,
}

#[allow(dead_code)]
impl MoveFileRequestBuilder {
    pub(crate) fn move_file<'a, S: FileDescriptor, T: FolderDescriptor>(
        client: &PCloudClient,
        file_like: S,
        target_folder_like: T,
    ) -> Result<MoveFileRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let source = file_like.to_file()?;
        let target = target_folder_like.to_folder()?;

        if !source.is_empty() && !target.is_empty() {
            Ok(MoveFileRequestBuilder {
                from_path: source.path,
                from_file_id: source.file_id,
                to_path: target.path,
                to_folder_id: target.folder_id,
                client: client.clone(),
                to_name: None,
                revision_id: source.revision,
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    /// name of the destination file. If omitted, then the original filename is used
    pub fn with_new_name(mut self, value: &str) -> MoveFileRequestBuilder {
        self.to_name = Some(value.to_string());
        self
    }

    /// Choose the revision of the file. If not set the latest revision is used.
    pub fn with_revision(mut self, value: u64) -> MoveFileRequestBuilder {
        self.revision_id = Some(value);
        self
    }

    // Execute the move operation
    pub async fn execute(
        self,
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error + Send + Sync>> {
        let mut r = self
            .client
            .client
            .post(format!("{}/renamefile", self.client.api_host));

        if let Some(v) = self.from_path {
            r = r.query(&[("path", v)]);
        }

        if let Some(v) = self.from_file_id {
            r = r.query(&[("fileid", v)]);
        }

        if let Some(v) = self.to_path {
            r = r.query(&[("topath", v)]);
        }

        if let Some(v) = self.to_folder_id {
            r = r.query(&[("tofolderid", v)]);
        }

        if let Some(v) = self.to_name {
            r = r.query(&[("toname", v)]);
        }

        if let Some(v) = self.revision_id {
            r = r.query(&[("revisionid", v)]);
        }

        r = self.client.add_token(r);

        let result = r
            .send()
            .await?
            .json::<pcloud_model::FileOrFolderStat>()
            .await?
            .assert_ok()?;
        Ok(result)
    }
}

pub struct UploadRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// Path of the target folder
    path: Option<String>,
    ///  id of the target folder
    folder_id: Option<u64>,
    /// If is set, partially uploaded files will not be saved
    no_partial: bool,
    /// if set, the uploaded file will be renamed, if file with the requested name exists in the folder.
    rename_if_exists: bool,
    /// if set, file modified time is set. Have to be unix time seconds.
    mtime: Option<i64>,
    /// if set, file created time is set. It's required to provide mtime to set ctime. Have to be unix time seconds.
    ctime: Option<i64>,
    /// files to upload
    files: Vec<reqwest::multipart::Part>,
}

#[allow(dead_code)]
impl UploadRequestBuilder {
    pub(crate) fn into_folder<'a, T: FolderDescriptor>(
        client: &PCloudClient,
        folder_like: T,
    ) -> Result<UploadRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let f = folder_like.to_folder()?;

        if !f.is_empty() {
            Ok(UploadRequestBuilder {
                folder_id: f.folder_id,
                path: f.path,
                client: client.clone(),
                no_partial: true,
                rename_if_exists: false,
                mtime: None,
                ctime: None,
                files: Vec::new(),
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    ///  If is set, partially uploaded files will not be saved (defaults to true)
    pub fn no_partial(mut self, value: bool) -> UploadRequestBuilder {
        self.no_partial = value;
        self
    }

    ///  if set, the uploaded file will be renamed, if file with the requested name exists in the folder.
    pub fn rename_if_exists(mut self, value: bool) -> UploadRequestBuilder {
        self.rename_if_exists = value;
        self
    }

    /// if set, file modified time is set. Have to be unix time seconds.
    pub fn mtime<Tz>(mut self, value: &DateTime<Tz>) -> UploadRequestBuilder
    where
        Tz: TimeZone,
        Tz::Offset: Display,
    {
        self.mtime = Some(value.timestamp());
        self
    }

    ///  if set, file created time is set. It's required to provide mtime to set ctime. Have to be unix time seconds.
    pub fn ctime<Tz>(mut self, value: &DateTime<Tz>) -> UploadRequestBuilder
    where
        Tz: TimeZone,
        Tz::Offset: Display,
    {
        self.ctime = Some(value.timestamp());
        self
    }

    /// Adds a file to the upload request. Multiple files can be added!
    pub fn with_file<T: Into<Body>>(mut self, file_name: &str, body: T) -> UploadRequestBuilder {
        let file_part = reqwest::multipart::Part::stream(body).file_name(file_name.to_string());
        self.files.push(file_part);
        self
    }

    // Finally uploads the files
    pub async fn upload(self) -> Result<UploadedFile, Box<dyn std::error::Error + Send + Sync>> {
        if self.files.is_empty() {
            // Short cut operation if no files are configured to upload
            debug!("Requested file upload, but no files are added to the request.");
            let result = UploadedFile {
                result: PCloudResult::Ok,
                fileids: Vec::default(),
                metadata: Vec::default(),
            };
            return Ok(result);
        }

        let mut r = self
            .client
            .client
            .post(format!("{}/uploadfile", self.client.api_host));

        if let Some(v) = self.path {
            r = r.query(&[("path", v)]);
        }

        if let Some(v) = self.folder_id {
            r = r.query(&[("folderid", v)]);
        }

        if self.no_partial {
            r = r.query(&[("nopartial", "1")]);
        }

        if self.rename_if_exists {
            r = r.query(&[("renameifexists", "1")]);
        }

        if let Some(v) = self.mtime {
            r = r.query(&[("mtime", v)]);
        }

        if let Some(v) = self.ctime {
            r = r.query(&[("ctime", v)]);
        }

        r = self.client.add_token(r);

        let mut form = reqwest::multipart::Form::new();
        for part in self.files {
            form = form.part("part", part);
        }

        r = r.multipart(form);

        let result = r.send().await?.json::<UploadedFile>().await?.assert_ok()?;
        Ok(result)
    }
}

pub struct PublicFileLinkRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// file id of the file for public link
    file_id: Option<u64>,
    /// path to the file for public link
    path: Option<String>,
    /// Datetime when the link will stop working
    expire: Option<String>,
    max_downloads: Option<u64>,
    max_traffic: Option<u64>,
    short_link: bool,
    link_password: Option<String>,
    /// File revision to fetch
    revision_id: Option<u64>,
}

#[allow(dead_code)]
impl PublicFileLinkRequestBuilder {
    pub(crate) fn for_file<'a, T: FileDescriptor>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<PublicFileLinkRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let f: PCloudFile = file_like.to_file()?;

        if !f.is_empty() {
            Ok(PublicFileLinkRequestBuilder {
                file_id: f.file_id,
                path: f.path,
                client: client.clone(),
                expire: None,
                max_downloads: None,
                max_traffic: None,
                short_link: false,
                link_password: None,
                revision_id: f.revision,
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    //  Datetime when the link will stop working
    pub fn expire_link_after<Tz>(mut self, value: &DateTime<Tz>) -> PublicFileLinkRequestBuilder
    where
        Tz: TimeZone,
        Tz::Offset: Display,
    {
        self.expire = Some(pcloud_model::format_date_time_for_pcloud(value));
        self
    }

    /// Maximum number of downloads for this file
    pub fn with_max_downloads(mut self, value: u64) -> PublicFileLinkRequestBuilder {
        self.max_downloads = Some(value);
        self
    }

    /// Maximum traffic that this link will consume (in bytes, started downloads will not be cut to fit in this limit)
    pub fn with_max_traffic(mut self, value: u64) -> PublicFileLinkRequestBuilder {
        self.max_traffic = Some(value);
        self
    }

    ///  If set, a short link will also be generated
    pub fn with_shortlink(mut self, value: bool) -> PublicFileLinkRequestBuilder {
        self.short_link = value;
        self
    }

    ///  Sets password for the link.
    pub fn with_password(mut self, value: &str) -> PublicFileLinkRequestBuilder {
        self.link_password = Some(value.to_string());
        self
    }

    /// Choose the revision of the file. If not set the latest revision is used.
    pub fn with_revision(mut self, value: u64) -> PublicFileLinkRequestBuilder {
        self.revision_id = Some(value);
        self
    }

    pub async fn get(self) -> Result<PublicFileLink, Box<dyn std::error::Error + Send + Sync>> {
        let mut r = self
            .client
            .client
            .get(format!("{}/getfilepublink", self.client.api_host));

        if let Some(id) = self.file_id {
            debug!("Requesting public link for file {}", id);
            r = r.query(&[("fileid", id)]);
        }

        if let Some(p) = self.path {
            debug!("Requesting public link for file {}", p);
            r = r.query(&[("path", p)]);
        }

        if let Some(v) = self.max_downloads {
            r = r.query(&[("maxdownloads", v)]);
        }

        if let Some(v) = self.link_password {
            r = r.query(&[("linkpassword", v)]);
        }

        if let Some(v) = self.max_traffic {
            r = r.query(&[("maxtraffic", v)]);
        }

        if self.short_link {
            r = r.query(&[("shortlink", "1")]);
        }

        if let Some(v) = self.expire {
            r = r.query(&[("expire", v)]);
        }

        if let Some(v) = self.revision_id {
            r = r.query(&[("revisionid", v)]);
        }

        r = self.client.add_token(r);

        let diff = r
            .send()
            .await?
            .json::<pcloud_model::PublicFileLink>()
            .await?
            .assert_ok()?;
        Ok(diff)
    }
}

pub(crate) struct PublicFileDownloadRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// either 'code' or 'shortcode'
    code: String,
    ///  File id, if the link is to a folder
    file_id: Option<u64>,
}

#[allow(dead_code)]
impl PublicFileDownloadRequestBuilder {
    /// Requests the download of a public file with a given code
    pub(crate) fn for_public_file(
        client: &PCloudClient,
        code: &str,
    ) -> PublicFileDownloadRequestBuilder {
        PublicFileDownloadRequestBuilder {
            code: code.to_string(),
            file_id: None,
            client: client.clone(),
        }
    }

    /// Requests a file from a public folder with a given code
    pub(crate) fn for_file_in_public_folder(
        client: &PCloudClient,
        code: &str,
        file_id: u64,
    ) -> PublicFileDownloadRequestBuilder {
        PublicFileDownloadRequestBuilder {
            code: code.to_string(),
            file_id: Some(file_id),
            client: client.clone(),
        }
    }

    /// Create file download link
    pub async fn get(
        self,
    ) -> Result<pcloud_model::DownloadLink, Box<dyn std::error::Error + Send + Sync>> {
        let mut r = self
            .client
            .client
            .get(format!("{}/getpublinkdownload", self.client.api_host));

        r = r.query(&[("code", self.code)]);

        if let Some(id) = self.file_id {
            r = r.query(&[("fileid", id)]);
        }

        r = self.client.add_token(r);

        let diff = r
            .send()
            .await?
            .json::<pcloud_model::DownloadLink>()
            .await?
            .assert_ok()?;
        Ok(diff)
    }
}

pub struct ListRevisionsRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    ///  ID of the  file
    file_id: Option<u64>,
    /// Path to the  file
    path: Option<String>,
}

impl ListRevisionsRequestBuilder {
    pub(crate) fn for_file<'a, T: FileDescriptor>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<ListRevisionsRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let f = file_like.to_file()?;

        if !f.is_empty() {
            Ok(ListRevisionsRequestBuilder {
                file_id: f.file_id,
                path: f.path,
                client: client.clone(),
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    /// Executes the request
    pub async fn get(self) -> Result<RevisionList, Box<dyn std::error::Error + Send + Sync>> {
        let mut r = self
            .client
            .client
            .get(format!("{}/listrevisions", self.client.api_host));

        if let Some(id) = self.file_id {
            debug!("Requesting file revisions for file {}", id);
            r = r.query(&[("fileid", id)]);
        }

        if let Some(p) = self.path {
            debug!("Requesting file revisions for file {}", p);
            r = r.query(&[("path", p)]);
        }

        r = self.client.add_token(r);

        let result = r.send().await?.json::<RevisionList>().await?.assert_ok()?;
        Ok(result)
    }
}

pub struct ChecksumFileRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    ///  ID of the  file
    file_id: Option<u64>,
    /// Path to the  file
    path: Option<String>,
    /// File revision to fetch
    revision_id: Option<u64>,
}

#[allow(dead_code)]
impl ChecksumFileRequestBuilder {
    pub(crate) fn for_file<'a, T: FileDescriptor>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<ChecksumFileRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let f = file_like.to_file()?;

        if !f.is_empty() {
            Ok(ChecksumFileRequestBuilder {
                file_id: f.file_id,
                path: f.path,
                client: client.clone(),
                revision_id: f.revision,
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    /// Choose the revision of the file. If not set the latest revision is used.
    pub fn with_revision(mut self, value: u64) -> ChecksumFileRequestBuilder {
        self.revision_id = Some(value);
        self
    }

    /// Executes the request
    pub async fn get(
        self,
    ) -> Result<pcloud_model::FileChecksums, Box<dyn std::error::Error + Send + Sync>> {
        let mut r = self
            .client
            .client
            .get(format!("{}/checksumfile", self.client.api_host));

        if let Some(id) = self.file_id {
            debug!("Requesting file checksums for file {}", id);
            r = r.query(&[("fileid", id)]);
        }

        if let Some(p) = self.path {
            debug!("Requesting file checksums for file {}", p);
            r = r.query(&[("path", p)]);
        }

        if let Some(v) = self.revision_id {
            r = r.query(&[("revisionid", v)]);
        }

        r = self.client.add_token(r);

        let diff = r
            .send()
            .await?
            .json::<pcloud_model::FileChecksums>()
            .await?
            .assert_ok()?;
        Ok(diff)
    }
}

pub struct FileDeleteRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    ///  ID of the  file
    file_id: Option<u64>,
    /// Path to the  file
    path: Option<String>,
}

#[allow(dead_code)]
impl FileDeleteRequestBuilder {
    pub(crate) fn for_file<'a, T: FileDescriptor>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<FileDeleteRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let f = file_like.to_file()?;

        if !f.is_empty() {
            Ok(FileDeleteRequestBuilder {
                file_id: f.file_id,
                path: f.path,
                client: client.clone(),
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    pub async fn execute(
        self,
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error + Send + Sync>> {
        let mut r = self
            .client
            .client
            .get(format!("{}/deletefile", self.client.api_host));

        if let Some(id) = self.file_id {
            debug!("Requesting delete for file {}", id);
            r = r.query(&[("fileid", id)]);
        }

        if let Some(p) = self.path {
            debug!("Requesting delete for file {}", p);
            r = r.query(&[("path", p)]);
        }

        r = self.client.add_token(r);

        let diff = r
            .send()
            .await?
            .json::<pcloud_model::FileOrFolderStat>()
            .await?
            .assert_ok()?;
        Ok(diff)
    }
}

pub struct FileDownloadRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    ///  ID of the  file
    file_id: Option<u64>,
    /// Path to the  file
    path: Option<String>,
    /// File revision to fetch
    revision_id: Option<u64>,
}

#[allow(dead_code)]
impl FileDownloadRequestBuilder {
    pub(crate) fn for_file<'a, T: FileDescriptor>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<FileDownloadRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let f = file_like.to_file()?;

        if !f.is_empty() {
            Ok(FileDownloadRequestBuilder {
                file_id: f.file_id,
                path: f.path,
                client: client.clone(),
                revision_id: f.revision,
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    /// Choose the revision of the file. If not set the latest revision is used.
    pub fn with_revision(mut self, value: u64) -> FileDownloadRequestBuilder {
        self.revision_id = Some(value);
        self
    }

    /// Fetch the download link for the file
    pub async fn get(
        self,
    ) -> Result<pcloud_model::DownloadLink, Box<dyn std::error::Error + Send + Sync>> {
        let mut r = self
            .client
            .client
            .get(format!("{}/getfilelink", self.client.api_host));

        if let Some(id) = self.file_id {
            debug!("Requesting download for file {}", id);
            r = r.query(&[("fileid", id)]);
        }

        if let Some(p) = self.path {
            debug!("Requesting download for file {}", p);
            r = r.query(&[("path", p)]);
        }

        if let Some(v) = self.revision_id {
            r = r.query(&[("revisionid", v)]);
        }

        r = self.client.add_token(r);

        let diff = r
            .send()
            .await?
            .json::<pcloud_model::DownloadLink>()
            .await?
            .assert_ok()?;
        Ok(diff)
    }
}

pub struct FileStatRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    ///  ID of the  file
    file_id: Option<u64>,
    /// Path to the  file
    path: Option<String>,
    /// File revision to fetch
    revision_id: Option<u64>,
}

#[allow(dead_code)]
impl FileStatRequestBuilder {
    pub(crate) fn for_file<'a, T: FileDescriptor>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<FileStatRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let f = file_like.to_file()?;

        if !f.is_empty() {
            Ok(FileStatRequestBuilder {
                file_id: f.file_id,
                path: f.path,
                client: client.clone(),
                revision_id: f.revision,
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    /// Choose the revision of the file. If not set the latest revision is used.
    pub fn with_revision(mut self, value: u64) -> FileStatRequestBuilder {
        self.revision_id = Some(value);
        self
    }

    /// Fetch the file metadata
    pub async fn get(
        self,
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error + Send + Sync>> {
        let mut r = self
            .client
            .client
            .get(format!("{}/stat", self.client.api_host));

        if let Some(id) = self.file_id {
            debug!("Requesting file metadata for file {}", id);
            r = r.query(&[("fileid", id)]);
        }

        if let Some(p) = self.path {
            debug!("Requesting file metadata for file {}", p);
            r = r.query(&[("path", p)]);
        }

        if let Some(v) = self.revision_id {
            r = r.query(&[("revisionid", v)]);
        }

        r = self.client.add_token(r);

        let diff = r
            .send()
            .await?
            .json::<pcloud_model::FileOrFolderStat>()
            .await?
            .assert_ok()?;
        Ok(diff)
    }
}

#[allow(dead_code)]
impl PCloudClient {
    /// Downloads a DownloadLink
    pub async fn download_link(
        &self,
        link: &pcloud_model::DownloadLink,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(url) = link.into_url() {
            debug!("Downloading file link {}", url);

            // No authentication necessary!
            // r = self.add_token(r);
            let resp = self.client.get(url).send().await?;

            Ok(resp)
        } else {
            Err(PCloudResult::ProvideURL)?
        }
    }

    /// Returns the file id (and the revision if given) of a PCloudFile. If the file_id is given, just return it. If a path is given, fetch the metadata with the file id.
    pub(crate) async fn get_file_id<T: FileDescriptor>(
        &self,
        file_like: T,
    ) -> Result<(u64, Option<u64>), Box<dyn std::error::Error + Send + Sync>> {
        let file = file_like.to_file()?;
        let rev = file.revision;

        if let Some(file_id) = file.file_id {
            Ok((file_id, rev))
        } else {
            let metadata = self.get_file_metadata(file).await?.metadata.unwrap();

            if metadata.isfolder {
                Err(PCloudResult::NoFileIdOrPathProvided)?
            }

            if let Some(file_id) = metadata.fileid {
                Ok((file_id, rev))
            } else {
                Err(PCloudResult::NoFileIdOrPathProvided)?
            }
        }
    }

    /// Fetches the download link for the latest file revision and directly downloads the file.  Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub async fn download_file<'a, T: FileDescriptor>(
        &self,
        file_like: T,
    ) -> Result<Response, Box<dyn 'a + std::error::Error + Send + Sync>> {
        let link = self.get_download_link_for_file(file_like)?.get().await?;
        self.download_link(&link).await
    }

    /// Copies the given file to the given folder. Either set a target folder id and then the target with with_new_name or give a full new file path as target path
    pub fn copy_file<'a, S: FileDescriptor, T: FolderDescriptor>(
        &self,
        file_like: S,
        target_folder_like: T,
    ) -> Result<CopyFileRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        CopyFileRequestBuilder::copy_file(self, file_like, target_folder_like)
    }

    /// Moves the given file to the given folder. Either set a target folder id and then the target with with_new_name or give a full new file path as target path
    pub fn move_file<'a, S: FileDescriptor, T: FolderDescriptor>(
        &self,
        file_like: S,
        target_folder_like: T,
    ) -> Result<MoveFileRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        MoveFileRequestBuilder::move_file(self, file_like, target_folder_like)
    }

    /// Lists revisions for a given fileid / path
    pub async fn list_file_revisions<'a, S: FileDescriptor>(
        &self,
        file_like: S,
    ) -> Result<RevisionList, Box<dyn 'a + std::error::Error + Send + Sync>> {
        ListRevisionsRequestBuilder::for_file(self, file_like)?
            .get()
            .await
    }

    /// Returns the metadata of a file. Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub async fn get_file_metadata<'a, T: FileDescriptor>(
        &self,
        file_like: T,
    ) -> Result<FileOrFolderStat, Box<dyn 'a + std::error::Error + Send + Sync>> {
        FileStatRequestBuilder::for_file(self, file_like)?
            .get()
            .await
    }

    /// Requests deleting a file. Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub async fn delete_file<'a, T: FileDescriptor>(
        &self,
        file_like: T,
    ) -> Result<FileOrFolderStat, Box<dyn 'a + std::error::Error + Send + Sync>> {
        FileDeleteRequestBuilder::for_file(self, file_like)?
            .execute()
            .await
    }

    /// Requests the checksums of a file. Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub fn checksum_file<'a, T: FileDescriptor>(
        &self,
        file_like: T,
    ) -> Result<ChecksumFileRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        ChecksumFileRequestBuilder::for_file(self, file_like)
    }

    /// Returns the public link for a pCloud file. Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub fn get_public_link_for_file<'a, T: FileDescriptor>(
        &self,
        file_like: T,
    ) -> Result<PublicFileLinkRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        PublicFileLinkRequestBuilder::for_file(&self, file_like)
    }

    /// Returns the public download link for a public file link
    pub async fn get_public_download_link_for_file(
        &self,
        link: &pcloud_model::PublicFileLink,
    ) -> Result<pcloud_model::DownloadLink, Box<dyn std::error::Error + Send + Sync>> {
        PublicFileDownloadRequestBuilder::for_public_file(self, link.code.clone().unwrap().as_str())
            .get()
            .await
    }

    /// Returns the download link for a file. Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub fn get_download_link_for_file<'a, T: FileDescriptor>(
        &self,
        file_like: T,
    ) -> Result<FileDownloadRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        FileDownloadRequestBuilder::for_file(self, file_like)
    }

    /// Uploads files into a folder. Accepts either a folder id (u64), a folder path (String) or any other pCloud object describing a folder (like Metadata)
    pub fn upload_file_into_folder<'a, T: FolderDescriptor>(
        &self,
        folder_like: T,
    ) -> Result<UploadRequestBuilder, Box<dyn 'a + std::error::Error + Send + Sync>> {
        UploadRequestBuilder::into_folder(self, folder_like)
    }

    /// Creates a Tree required for some requests (like building a zip file)
    pub fn create_tree(&self) -> Tree {
        Tree::create(self)
    }
}
