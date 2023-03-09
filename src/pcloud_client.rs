use std::fmt::Display;

use crate::pcloud_model::{
    self, Diff, FileChecksums, FileOrFolderStat, Metadata, PCloudResult, PublicFileLink,
    UploadedFile, UserInfo, WithPCloudResult,
};
use chrono::{DateTime, TimeZone};
use log::{debug, warn};
use reqwest::{Body, Client, RequestBuilder, Response};

/// Generic description of a PCloud File. Either by its file id (preferred) or by its path
pub struct PCloudFile {
    /// ID of the target file
    file_id: Option<u64>,
    /// Path of the target file
    path: Option<String>,
}

/// Convert Strings into pCloud file paths
impl Into<PCloudFile> for &str {
    fn into(self) -> PCloudFile {
        PCloudFile {
            file_id: None,
            path: Some(self.to_string()),
        }
    }
}

/// Convert Strings into pCloud file paths
impl Into<PCloudFile> for String {
    fn into(self) -> PCloudFile {
        PCloudFile {
            file_id: None,
            path: Some(self),
        }
    }
}

/// Convert u64 into pCloud file ids
impl Into<PCloudFile> for u64 {
    fn into(self) -> PCloudFile {
        PCloudFile {
            file_id: Some(self),
            path: None,
        }
    }
}

/// Convert u64 into pCloud file ids
impl Into<PCloudFile> for &u64 {
    fn into(self) -> PCloudFile {
        PCloudFile {
            file_id: Some(self.clone()),
            path: None,
        }
    }
}

/// Extract file id from pCloud file or folder metadata response
impl TryInto<PCloudFile> for &Metadata {
    type Error = PCloudResult;

    fn try_into(self) -> Result<PCloudFile, PCloudResult> {
        if self.isfolder {
            Err(PCloudResult::InvalidFileOrFolderName)?
        } else {
            Ok(PCloudFile {
                file_id: self.fileid,
                path: None,
            })
        }
    }
}

/// Extract file id from pCloud file or folder metadata response
impl TryInto<PCloudFile> for &FileOrFolderStat {
    type Error = PCloudResult;
    fn try_into(self) -> Result<PCloudFile, PCloudResult> {
        if self.result == PCloudResult::Ok && self.metadata.is_some() {
            let metadata = self.metadata.as_ref().unwrap();
            metadata.try_into()
        } else {
            Err(PCloudResult::InvalidFileOrFolderName)?
        }
    }
}

/// Generic description of a PCloud folder. Either by its file id (preferred) or by its path
pub struct PCloudFolder {
    /// ID of the target folder
    pub folder_id: Option<u64>,
    /// Path of the target folder
    pub path: Option<String>,
}

/// Convert Strings into pCloud folder paths
impl TryInto<PCloudFolder> for &str {
    type Error = PCloudResult;

    fn try_into(self) -> Result<PCloudFolder, PCloudResult> {
        if self == "/" {
            // Root folder has always id 0
            Ok(PCloudFolder {
                folder_id: Some(0),
                path: None,
            })
        } else if self.starts_with("/") {
            // File paths must always be absolute paths
            Ok(PCloudFolder {
                folder_id: None,
                path: Some(self.to_string()),
            })
        } else {
            Err(PCloudResult::InvalidPath)?
        }
    }
}

/// Convert Strings into pCloud folder paths
impl TryInto<PCloudFolder> for String {
    type Error = PCloudResult;

    fn try_into(self) -> Result<PCloudFolder, PCloudResult> {
        if self == "/" {
            // Root folder has always id 0
            Ok(PCloudFolder {
                folder_id: Some(0),
                path: None,
            })
        } else if self.starts_with("/") {
            // File paths must always be absolute paths
            Ok(PCloudFolder {
                folder_id: None,
                path: Some(self),
            })
        } else {
            Err(PCloudResult::InvalidPath)?
        }
    }
}

/// Convert u64 into pCloud folder ids
impl Into<PCloudFolder> for u64 {
    fn into(self) -> PCloudFolder {
        PCloudFolder {
            folder_id: Some(self),
            path: None,
        }
    }
}

/// Convert u64 into pCloud folder ids
impl Into<PCloudFolder> for &u64 {
    fn into(self) -> PCloudFolder {
        PCloudFolder {
            folder_id: Some(self.clone()),
            path: None,
        }
    }
}

/// Extract file id from pCloud folder metadata
impl TryInto<PCloudFolder> for &Metadata {
    type Error = PCloudResult;

    fn try_into(self) -> Result<PCloudFolder, PCloudResult> {
        if !self.isfolder {
            Err(PCloudResult::InvalidFileOrFolderName)?
        } else {
            Ok(PCloudFolder {
                folder_id: self.folderid,
                path: None,
            })
        }
    }
}

/// Extract folder id from pCloud file or folder metadata response
impl TryInto<PCloudFolder> for &FileOrFolderStat {
    type Error = PCloudResult;

    fn try_into(self) -> Result<PCloudFolder, PCloudResult> {
        if self.result == PCloudResult::Ok && self.metadata.is_some() {
            let metadata = self.metadata.as_ref().unwrap();
            metadata.try_into()
        } else {
            Err(PCloudResult::InvalidPath)?
        }
    }
}

pub struct DeleteFolderRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// Path of the folder
    path: Option<String>,
    ///  id of the folder
    folder_id: Option<u64>,
}

#[allow(dead_code)]
impl DeleteFolderRequestBuilder {
    fn for_folder<'a, T: TryInto<PCloudFolder>>(
        client: &PCloudClient,
        folder_like: T,
    ) -> Result<DeleteFolderRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        let f = folder_like.try_into()?;

        if f.folder_id.is_some() || f.path.is_some() {
            Ok(DeleteFolderRequestBuilder {
                folder_id: f.folder_id,
                path: f.path,
                client: client.clone(),
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    /// Deletes the folder and all its content recursively
    pub async fn delete_recursive(
        self,
    ) -> Result<pcloud_model::FolderRecursivlyDeleted, Box<dyn std::error::Error>> {
        let url = format!("{}/deletefolderrecursive", self.client.api_host);

        let mut r = self.client.client.get(url);

        if let Some(p) = self.path {
            debug!("Deleting folder {} recursively", p);
            r = r.query(&[("path", p)]);
        }

        if let Some(id) = self.folder_id {
            debug!("Deleting folder with {} recursively", id);
            r = r.query(&[("folderid", id)]);
        }

        r = self.client.add_token(r);

        let stat = r
            .send()
            .await?
            .json::<pcloud_model::FolderRecursivlyDeleted>()
            .await?
            .assert_ok()?;
        Ok(stat)
    }

    /// Deletes the folder, only if  it is empty
    pub async fn delete_folder_if_empty(
        self,
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error>> {
        let url = format!("{}/deletefolder", self.client.api_host);

        let mut r = self.client.client.get(url);

        if let Some(p) = self.path {
            debug!("Deleting folder {} if empty", p);
            r = r.query(&[("path", p)]);
        }

        if let Some(id) = self.folder_id {
            debug!("Deleting folder with {} if empty", id);
            r = r.query(&[("folderid", id)]);
        }

        r = self.client.add_token(r);

        let stat = r
            .send()
            .await?
            .json::<pcloud_model::FileOrFolderStat>()
            .await?
            .assert_ok()?;
        Ok(stat)
    }
}

pub struct CreateFolderRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// Path of the parent folder
    path: Option<String>,
    ///  id of the parent folder
    folder_id: Option<u64>,
    /// Name of the folder to create
    name: String,
    /// Creates a folder if the folder doesn't exist or returns the existing folder's metadata.
    if_not_exists: bool,
}

#[allow(dead_code)]
impl CreateFolderRequestBuilder {
    fn for_folder<'a, T: TryInto<PCloudFolder>>(
        client: &PCloudClient,
        folder_like_parent: T,
        name: &str,
    ) -> Result<CreateFolderRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        let f = folder_like_parent.try_into()?;

        if f.folder_id.is_some() || f.path.is_some() {
            Ok(CreateFolderRequestBuilder {
                folder_id: f.folder_id,
                path: f.path,
                client: client.clone(),
                name: name.to_string(),
                if_not_exists: true,
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    /// If true (default), creates a folder if the folder doesn't exist or returns the existing folder's metadata. If false, creating of the folder fails
    pub fn if_not_exists(mut self, value: bool) -> CreateFolderRequestBuilder {
        self.if_not_exists = value;
        self
    }

    /// Creates the folder
    pub async fn execute(
        self,
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error>> {
        let url = if self.if_not_exists {
            format!("{}/createfolderifnotexists", self.client.api_host)
        } else {
            format!("{}/createfolder", self.client.api_host)
        };

        let mut r = self.client.client.get(url);

        if let Some(p) = self.path {
            debug!("Creating folder {} in folder {}", self.name, p);
            r = r.query(&[("path", p)]);
        }

        if let Some(id) = self.folder_id {
            debug!("Creating folder {} in folder {}", self.name, id);
            r = r.query(&[("folderid", id)]);
        }

        r = r.query(&[("name", self.name)]);

        r = self.client.add_token(r);

        let stat = r
            .send()
            .await?
            .json::<pcloud_model::FileOrFolderStat>()
            .await?
            .assert_ok()?;
        Ok(stat)
    }
}

pub struct CopyFolderRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// source file path
    from_path: Option<String>,
    /// source file id
    from_folder_id: Option<u64>,
    /// destination folder path
    to_path: Option<String>,
    /// destination folder id
    to_folder_id: Option<u64>,
    /// New file name
    to_name: Option<String>,
    /// If it is set and files with the same name already exist, overwriting will be preformed (otherwise error 2004 will be returned)
    overwrite: bool,
    /// If set will skip files that already exist
    skipexisting: bool,
    ///  If it is set only the content of source folder will be copied otherwise the folder itself is copied
    copycontentonly: bool,
}

#[allow(dead_code)]
impl CopyFolderRequestBuilder {
    /// Copies a folder identified by folderid or path to either topath or tofolderid.
    fn copy_folder<'a, S: TryInto<PCloudFolder>, T: TryInto<PCloudFolder>>(
        client: &PCloudClient,
        folder_like: S,
        target_folder_like: T,
    ) -> Result<CopyFolderRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
        S::Error: 'a + std::error::Error,
    {
        let source: PCloudFolder = folder_like.try_into()?;
        let target: PCloudFolder = target_folder_like.try_into()?;

        if (source.folder_id.is_some() || source.path.is_some())
            && (target.folder_id.is_some() || target.path.is_some())
        {
            Ok(CopyFolderRequestBuilder {
                from_path: source.path,
                from_folder_id: source.folder_id,
                to_path: target.path,
                to_folder_id: target.folder_id,
                client: client.clone(),
                to_name: None,
                overwrite: true,
                skipexisting: false,
                copycontentonly: false,
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    /// If it is set (default true) and files with the same name already exist, overwriting will be preformed (otherwise error 2004 will be returned)
    pub fn overwrite(mut self, value: bool) -> CopyFolderRequestBuilder {
        self.overwrite = value;
        self
    }

    /// If set will skip files that already exist
    pub fn skipexisting(mut self, value: bool) -> CopyFolderRequestBuilder {
        self.skipexisting = value;
        self
    }

    /// If it is set only the content of source folder will be copied otherwise the folder itself is copied
    pub fn copycontentonly(mut self, value: bool) -> CopyFolderRequestBuilder {
        self.copycontentonly = value;
        self
    }

    /// Execute the copy operation
    pub async fn execute(
        self,
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error>> {
        let mut r = self
            .client
            .client
            .post(format!("{}/copyfolder", self.client.api_host));

        if let Some(v) = self.from_path {
            r = r.query(&[("path", v)]);
        }

        if let Some(v) = self.from_folder_id {
            r = r.query(&[("folderid", v)]);
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

        if !self.overwrite {
            r = r.query(&[("noover", "1")]);
        }

        if !self.skipexisting {
            r = r.query(&[("skipexisting", "1")]);
        }

        if !self.copycontentonly {
            r = r.query(&[("copycontentonly", "1")]);
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

pub struct MoveFolderRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// source file path
    from_path: Option<String>,
    /// source file id
    from_folder_id: Option<u64>,
    /// destination folder path
    to_path: Option<String>,
    /// destination folder id
    to_folder_id: Option<u64>,
    /// New file name
    to_name: Option<String>,
}

#[allow(dead_code)]
impl MoveFolderRequestBuilder {
    /// Renames (and/or moves) a folder identified by folderid or path to either topath (if topath is a existing folder to place source folder without new name for the folder it MUST end with slash - /newpath/) or tofolderid/toname (one or both can be provided).
    fn move_folder<'a, S: TryInto<PCloudFolder>, T: TryInto<PCloudFolder>>(
        client: &PCloudClient,
        folder_like: S,
        target_folder_like: T,
    ) -> Result<MoveFolderRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
        S::Error: 'a + std::error::Error,
    {
        let source: PCloudFolder = folder_like.try_into()?;
        let target: PCloudFolder = target_folder_like.try_into()?;

        if (source.folder_id.is_some() || source.path.is_some())
            && (target.folder_id.is_some() || target.path.is_some())
        {
            Ok(MoveFolderRequestBuilder {
                from_path: source.path,
                from_folder_id: source.folder_id,
                to_path: target.path,
                to_folder_id: target.folder_id,
                client: client.clone(),
                to_name: None,
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    /// name of the destination file. If omitted, then the original filename is used
    pub fn with_new_name(mut self, value: &str) -> MoveFolderRequestBuilder {
        self.to_name = Some(value.to_string());
        self
    }

    // Execute the move operation
    pub async fn execute(
        self,
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error>> {
        let mut r = self
            .client
            .client
            .post(format!("{}/renamefolder", self.client.api_host));

        if let Some(v) = self.from_path {
            r = r.query(&[("path", v)]);
        }

        if let Some(v) = self.from_folder_id {
            r = r.query(&[("folderid", v)]);
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
}

#[allow(dead_code)]
impl CopyFileRequestBuilder {
    fn copy_file<'a, S: TryInto<PCloudFile>, T: TryInto<PCloudFolder>>(
        client: &PCloudClient,
        file_like: S,
        target_folder_like: T,
    ) -> Result<CopyFileRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
        S::Error: 'a + std::error::Error,
    {
        let source: PCloudFile = file_like.try_into()?;
        let target: PCloudFolder = target_folder_like.try_into()?;

        if (source.file_id.is_some() || source.path.is_some())
            && (target.folder_id.is_some() || target.path.is_some())
        {
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

    // Execute the copy operation
    pub async fn execute(
        self,
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error>> {
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
}

#[allow(dead_code)]
impl MoveFileRequestBuilder {
    fn move_file<'a, S: TryInto<PCloudFile>, T: TryInto<PCloudFolder>>(
        client: &PCloudClient,
        file_like: S,
        target_folder_like: T,
    ) -> Result<MoveFileRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
        S::Error: 'a + std::error::Error,
    {
        let source: PCloudFile = file_like.try_into()?;
        let target: PCloudFolder = target_folder_like.try_into()?;

        if (source.file_id.is_some() || source.path.is_some())
            && (target.folder_id.is_some() || target.path.is_some())
        {
            Ok(MoveFileRequestBuilder {
                from_path: source.path,
                from_file_id: source.file_id,
                to_path: target.path,
                to_folder_id: target.folder_id,
                client: client.clone(),
                to_name: None,
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

    // Execute the move operation
    pub async fn execute(
        self,
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error>> {
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
    fn into_folder<'a, T: TryInto<PCloudFolder>>(
        client: &PCloudClient,
        folder_like: T,
    ) -> Result<UploadRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        let f = folder_like.try_into()?;

        if f.folder_id.is_some() || f.path.is_some() {
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
    pub async fn upload(self) -> Result<UploadedFile, Box<dyn std::error::Error>> {
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

pub struct ListFolderRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// Path of the folder
    path: Option<String>,
    ///  id of the folder
    folder_id: Option<u64>,
    /// If is set full directory tree will be returned, which means that all directories will have contents filed.
    recursive: bool,
    ///  If is set, deleted files and folders that can be undeleted will be displayed.
    showdeleted: bool,
    ///  If is set, only the folder (sub)structure will be returned.
    nofiles: bool,
    /// If is set, only user's own folders and files will be displayed.
    noshares: bool,
}

#[allow(dead_code)]
impl ListFolderRequestBuilder {
    fn for_folder<'a, T: TryInto<PCloudFolder>>(
        client: &PCloudClient,
        folder_like: T,
    ) -> Result<ListFolderRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        let f = folder_like.try_into()?;

        if f.folder_id.is_some() || f.path.is_some() {
            Ok(ListFolderRequestBuilder {
                folder_id: f.folder_id,
                path: f.path,
                client: client.clone(),
                recursive: false,
                showdeleted: false,
                nofiles: false,
                noshares: false,
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    /// If is set full directory tree will be returned, which means that all directories will have contents filed.
    pub fn recursive(mut self, value: bool) -> ListFolderRequestBuilder {
        self.recursive = value;
        self
    }

    ///  If is set, deleted files and folders that can be undeleted will be displayed.
    pub fn showdeleted(mut self, value: bool) -> ListFolderRequestBuilder {
        self.showdeleted = value;
        self
    }

    ///  If is set, only the folder (sub)structure will be returned.
    pub fn nofiles(mut self, value: bool) -> ListFolderRequestBuilder {
        self.nofiles = value;
        self
    }

    /// If is set, only user's own folders and files will be displayed.
    pub fn noshares(mut self, value: bool) -> ListFolderRequestBuilder {
        self.noshares = value;
        self
    }

    /// Execute list operation
    pub async fn get(self) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error>> {
        let mut r = self
            .client
            .client
            .get(format!("{}/listfolder", self.client.api_host));

        if let Some(v) = self.path {
            debug!("List folder {}", v);
            r = r.query(&[("path", v)]);
        }

        if let Some(v) = self.folder_id {
            debug!("List folder {}", v);
            r = r.query(&[("folderid", v)]);
        }

        if self.recursive {
            r = r.query(&[("recursive", "1")]);
        }

        if self.showdeleted {
            r = r.query(&[("showdeleted", "1")]);
        }

        if self.nofiles {
            r = r.query(&[("nofiles", "1")]);
        }

        if self.noshares {
            r = r.query(&[("noshares", "1")]);
        }

        r = self.client.add_token(r);

        let stat = r
            .send()
            .await?
            .json::<pcloud_model::FileOrFolderStat>()
            .await?
            .assert_ok()?;
        Ok(stat)
    }
}

pub struct DiffRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    /// receive only changes since that diffid.
    diff_id: Option<u64>,
    /// datetime receive only events generated after that time
    after: Option<String>,
    /// return last number of events with highest diffids (that is the last events)
    last: Option<u64>,
    /// if set, the connection will block until an event arrives. Works only with diffid
    block: bool,
    /// if provided, no more than limit entries will be returned
    limit: Option<u64>,
}

#[allow(dead_code)]
impl DiffRequestBuilder {
    fn create(client: &PCloudClient) -> DiffRequestBuilder {
        DiffRequestBuilder {
            diff_id: None,
            after: None,
            last: None,
            block: false,
            limit: None,
            client: client.clone(),
        }
    }

    /// receive only changes since that diffid.
    pub fn after_diff_id(mut self, value: u64) -> DiffRequestBuilder {
        self.diff_id = Some(value);
        self
    }
    /// datetime receive only events generated after that time
    pub fn after<Tz>(mut self, value: &DateTime<Tz>) -> DiffRequestBuilder
    where
        Tz: TimeZone,
        Tz::Offset: Display,
    {
        self.after = Some(pcloud_model::format_date_time_for_pcloud(value));
        self
    }

    ///  return last number of events with highest diffids (that is the last events)
    pub fn only_last(mut self, value: u64) -> DiffRequestBuilder {
        self.last = Some(value);
        self
    }

    /// if set, the connection will block until an event arrives. Works only with diffid
    pub fn block(mut self, value: bool) -> DiffRequestBuilder {
        self.block = value;
        self
    }
    /// if provided, no more than limit entries will be returned
    pub fn limit(mut self, value: u64) -> DiffRequestBuilder {
        self.limit = Some(value);
        self
    }

    pub async fn get(self) -> Result<Diff, Box<dyn std::error::Error>> {
        let url = format!("{}/diff", self.client.api_host);
        let mut r = self.client.client.get(url);

        if let Some(v) = self.diff_id {
            r = r.query(&[("diffid", v)]);
        }

        if let Some(v) = self.last {
            r = r.query(&[("last", v)]);
        }

        if let Some(v) = self.limit {
            r = r.query(&[("limit", v)]);
        }

        if self.block {
            r = r.query(&[("block", "1")]);
        }

        if let Some(v) = self.after {
            r = r.query(&[("after", v)]);
        }

        r = self.client.add_token(r);

        let diff = r.send().await?.json::<pcloud_model::Diff>().await?;
        Ok(diff)
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
}

#[allow(dead_code)]
impl PublicFileLinkRequestBuilder {
    fn for_file<'a, T: TryInto<PCloudFile>>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<PublicFileLinkRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        let f: PCloudFile = file_like.try_into()?;

        if f.file_id.is_some() || f.path.is_some() {
            Ok(PublicFileLinkRequestBuilder {
                file_id: f.file_id,
                path: f.path,
                client: client.clone(),
                expire: None,
                max_downloads: None,
                max_traffic: None,
                short_link: false,
                link_password: None,
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

    pub async fn get(self) -> Result<PublicFileLink, Box<dyn std::error::Error>> {
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

pub struct PublicFileDownloadRequestBuilder {
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
    fn for_public_file(client: &PCloudClient, code: &str) -> PublicFileDownloadRequestBuilder {
        PublicFileDownloadRequestBuilder {
            code: code.to_string(),
            file_id: None,
            client: client.clone(),
        }
    }

    /// Requests a file from a public folder with a given code
    fn for_file_in_public_folder(
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
    pub async fn get(self) -> Result<pcloud_model::DownloadLink, Box<dyn std::error::Error>> {
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

pub struct ChecksumFileRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    ///  ID of the  file
    file_id: Option<u64>,
    /// Path to the  file
    path: Option<String>,
}

#[allow(dead_code)]
impl ChecksumFileRequestBuilder {
    fn for_file<'a, T: TryInto<PCloudFile>>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<ChecksumFileRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        let f = file_like.try_into()?;

        if f.file_id.is_some() || f.path.is_some() {
            Ok(ChecksumFileRequestBuilder {
                file_id: f.file_id,
                path: f.path,
                client: client.clone(),
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    pub async fn get(self) -> Result<pcloud_model::FileChecksums, Box<dyn std::error::Error>> {
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
    fn for_file<'a, T: TryInto<PCloudFile>>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<FileDeleteRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        let f = file_like.try_into()?;

        if f.file_id.is_some() || f.path.is_some() {
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
    ) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error>> {
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

struct FileDownloadRequestBuilder {
    /// Client to actually perform the request
    client: PCloudClient,
    ///  ID of the  file
    file_id: Option<u64>,
    /// Path to the  file
    path: Option<String>,
}

#[allow(dead_code)]
impl FileDownloadRequestBuilder {
    fn for_file<'a, T: TryInto<PCloudFile>>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<FileDownloadRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        let f = file_like.try_into()?;

        if f.file_id.is_some() || f.path.is_some() {
            Ok(FileDownloadRequestBuilder {
                file_id: f.file_id,
                path: f.path,
                client: client.clone(),
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    pub async fn get(self) -> Result<pcloud_model::DownloadLink, Box<dyn std::error::Error>> {
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
}

#[allow(dead_code)]
impl FileStatRequestBuilder {
    fn for_file<'a, T: TryInto<PCloudFile>>(
        client: &PCloudClient,
        file_like: T,
    ) -> Result<FileStatRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        let f = file_like.try_into()?;

        if f.file_id.is_some() || f.path.is_some() {
            Ok(FileStatRequestBuilder {
                file_id: f.file_id,
                path: f.path,
                client: client.clone(),
            })
        } else {
            Err(pcloud_model::PCloudResult::NoFileIdOrPathProvided)?
        }
    }

    pub async fn get(self) -> Result<pcloud_model::FileOrFolderStat, Box<dyn std::error::Error>> {
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

#[derive(Clone)]
pub struct PCloudClient {
    api_host: String,
    client: reqwest::Client,
    /// Session auth token (not the OAuth2 token, which is set as default header). Common for all copies of this PCloudClient
    session_token: std::sync::Arc<Option<PCloudClientSession>>,
}

/// Contains the client session opened on login (not necessary for oauth2 sessions)
/// Due to drop implementation, logout automatically happens once the sessions drops
#[derive(Clone, Debug)]
struct PCloudClientSession {
    /// Auth token (not the OAuth2 token, which is set as default header)
    token: String,
    /// Host to connect to pCloud API
    api_host: String,
    /// Client to connect
    client: reqwest::Client,
}

impl PCloudClientSession {
    /// Adds the session token to the query build
    fn add_token(&self, r: RequestBuilder) -> RequestBuilder {
        let token = self.token.clone();
        let result = r.query(&[("auth", token)]);
        return result;
    }
}

impl Drop for PCloudClientSession {
    /// Drop the aquired session token
    fn drop(&mut self) {
        let client = self.client.clone();
        let api_host = self.api_host.clone();
        let token = self.token.clone();

        let op = tokio::spawn(async move {
            let result = PCloudClient::logout(&client, &api_host, &token).await;

            match result {
                Ok(v) => {
                    if v {
                        debug!("Successful logout");
                    } else {
                        warn!("Failed to logout");
                    }
                    return v;
                }
                Err(_) => {
                    warn!("Error on logout");
                    return false;
                }
            }
        });
        // Wait until the lockout thread finished
        futures::executor::block_on(op).unwrap();
    }
}

#[allow(dead_code)]
impl PCloudClient {
    /// Creates a new PCloudClient instance with an already present OAuth 2.0 authentication token. Automatically determines nearest API server for best performance
    pub async fn with_oauth(
        host: &str,
        oauth2: &str,
    ) -> Result<PCloudClient, Box<dyn std::error::Error>> {
        let builder = reqwest::ClientBuilder::new();

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(format!("Bearer {}", oauth2).as_str()).unwrap(),
        );

        let client = builder.default_headers(headers).build().unwrap();

        let best_host = PCloudClient::get_best_api_server(&client, host, None).await?;

        Ok(PCloudClient {
            api_host: best_host,
            client: client,
            session_token: std::sync::Arc::new(None),
        })
    }

    /// Creates a new PCloudClient instance using username and password to obtain a temporary auth token. Token is revoked on drop of this instance.
    pub async fn with_username_and_password(
        host: &str,
        username: &str,
        password: &str,
    ) -> Result<PCloudClient, Box<dyn std::error::Error>> {
        let token = PCloudClient::login(host, username, password).await?;

        let builder = reqwest::ClientBuilder::new();

        let client = builder.build().unwrap();

        let best_host =
            PCloudClient::get_best_api_server(&client, host, Some(token.clone())).await?;

        let session = PCloudClientSession {
            api_host: best_host.clone(),
            client: client.clone(),
            token: token,
        };

        Ok(PCloudClient {
            api_host: best_host,
            client: client,
            session_token: std::sync::Arc::new(Some(session)),
        })
    }

    /// Performs the login to pCloud using username and password.
    async fn login(
        host: &str,
        username: &str,
        password: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/userinfo?getauth=1", host);

        let client = reqwest::ClientBuilder::new().build()?;

        let mut r = client.get(url);

        r = r.query(&[("username", username)]);
        r = r.query(&[("password", password)]);

        let userinfo = r.send().await?.json::<pcloud_model::UserInfo>().await?;

        if userinfo.result == PCloudResult::Ok && userinfo.auth.is_some() {
            debug!("Successfull login for user {}", username);
            Ok(userinfo.auth.unwrap())
        } else {
            Err(PCloudResult::AccessDenied)?
        }
    }

    /// Performs the logout for the token aquired with login
    async fn logout(
        client: &Client,
        api_host: &str,
        token: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut r = client.get(format!("{}/logout", api_host));

        r = r.query(&[("auth", token)]);

        let response = r
            .send()
            .await?
            .json::<pcloud_model::LogoutResponse>()
            .await?;

        Ok(response.result == PCloudResult::Ok
            && response.auth_deleted.is_some()
            && response.auth_deleted.unwrap())
    }

    /// If theres is a session token present, add it to the given request.
    fn add_token(&self, r: RequestBuilder) -> RequestBuilder {
        let arc = self.session_token.clone();

        if let Some(ref session) = *arc {
            return session.add_token(r);
        }

        return r;
    }

    // Determine fastest api server for the given default api server (either api.pcloud.com or eapi.pcloud.com)
    async fn get_best_api_server(
        client: &reqwest::Client,
        host: &str,
        session_token: Option<String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/getapiserver", host);

        let mut r = client.get(url);

        r = r.query(&[("auth", session_token)]);

        let api_servers = r.send().await?.json::<pcloud_model::ApiServers>().await?;

        let best_host = match api_servers.result {
            pcloud_model::PCloudResult::Ok => {
                let best_host_url = api_servers.api.get(0).unwrap();
                debug!(
                    "Found nearest pCloud API endpoint https://{} for default endpoint {}",
                    best_host_url, host
                );
                format!("https://{}", best_host_url)
            }
            _ => host.to_string(),
        };

        Ok(best_host)
    }

    /// List updates of the user's folders/files.
    pub fn diff(&self) -> DiffRequestBuilder {
        DiffRequestBuilder::create(self)
    }

    /// Lists the content of a folder. Accepts either a folder id (u64), a folder path (String) or any other pCloud object describing a folder (like Metadata)
    pub fn list_folder<'a, T: TryInto<PCloudFolder>>(
        &self,
        folder_like: T,
    ) -> Result<ListFolderRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        ListFolderRequestBuilder::for_folder(self, folder_like)
    }

    /// Uploads files into a folder. Accepts either a folder id (u64), a folder path (String) or any other pCloud object describing a folder (like Metadata)
    pub fn upload_file_into_folder<'a, T: TryInto<PCloudFolder>>(
        &self,
        folder_like: T,
    ) -> Result<UploadRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        UploadRequestBuilder::into_folder(self, folder_like)
    }

    /// Creates a new folder in a parent folder. Accepts either a folder id (u64), a folder path (String) or any other pCloud object describing a folder (like Metadata)
    pub fn create_folder<'a, T: TryInto<PCloudFolder>>(
        &self,
        parent_folder_like: T,
        name: &str,
    ) -> Result<CreateFolderRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        CreateFolderRequestBuilder::for_folder(self, parent_folder_like, name)
    }

    /// Deletes a folder. Either only if empty or recursively. Accepts either a folder id (u64), a folder path (String) or any other pCloud object describing a folder (like Metadata)
    pub fn delete_folder<'a, T: TryInto<PCloudFolder>>(
        &self,
        folder_like: T,
    ) -> Result<DeleteFolderRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        DeleteFolderRequestBuilder::for_folder(self, folder_like)
    }

    /// Copies a folder identified by folderid or path to either topath or tofolderid.
    pub fn copy_folder<'a, S: TryInto<PCloudFolder>, T: TryInto<PCloudFolder>>(
        &self,
        folder_like: S,
        target_folder_like: T,
    ) -> Result<CopyFolderRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        S::Error: 'a + std::error::Error,
        T::Error: 'a + std::error::Error,
    {
        CopyFolderRequestBuilder::copy_folder(self, folder_like, target_folder_like)
    }

    /// Renames (and/or moves) a folder identified by folderid or path to either topath (if topath is a existing folder to place source folder without new name for the folder it MUST end with slash - /newpath/) or tofolderid/toname (one or both can be provided).
    pub fn move_folder<'a, S: TryInto<PCloudFolder>, T: TryInto<PCloudFolder>>(
        &self,
        folder_like: S,
        target_folder_like: T,
    ) -> Result<MoveFolderRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        S::Error: 'a + std::error::Error,
        T::Error: 'a + std::error::Error,
    {
        MoveFolderRequestBuilder::move_folder(self, folder_like, target_folder_like)
    }

    /// Copies the given file to the given folder. Either set a target folder id and then the target with with_new_name or give a full new file path as target path
    pub fn copy_file<'a, S: TryInto<PCloudFile>, T: TryInto<PCloudFolder>>(
        &self,
        file_like: S,
        target_folder_like: T,
    ) -> Result<CopyFileRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        S::Error: 'a + std::error::Error,
        T::Error: 'a + std::error::Error,
    {
        CopyFileRequestBuilder::copy_file(self, file_like, target_folder_like)
    }

    /// Moves the given file to the given folder. Either set a target folder id and then the target with with_new_name or give a full new file path as target path
    pub fn move_file<'a, S: TryInto<PCloudFile>, T: TryInto<PCloudFolder>>(
        &self,
        file_like: S,
        target_folder_like: T,
    ) -> Result<MoveFileRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        S::Error: 'a + std::error::Error,
        T::Error: 'a + std::error::Error,
    {
        MoveFileRequestBuilder::move_file(self, file_like, target_folder_like)
    }

    /// Returns the metadata of a file. Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub async fn get_file_metadata<'a, T: TryInto<PCloudFile>>(
        &self,
        file_like: T,
    ) -> Result<FileOrFolderStat, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        FileStatRequestBuilder::for_file(self, file_like)?
            .get()
            .await
    }

    /// Requests deleting a file. Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub async fn delete_file<'a, T: TryInto<PCloudFile>>(
        &self,
        file_like: T,
    ) -> Result<FileOrFolderStat, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        FileDeleteRequestBuilder::for_file(self, file_like)?
            .execute()
            .await
    }

    /// Requests the checksums of a file. Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub async fn checksum_file<'a, T: TryInto<PCloudFile>>(
        &self,
        file_like: T,
    ) -> Result<FileChecksums, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        ChecksumFileRequestBuilder::for_file(self, file_like)?
            .get()
            .await
    }

    /// Returns the public link for a pCloud file. Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub fn get_public_link_for_file<'a, T: TryInto<PCloudFile>>(
        &self,
        file_like: T,
    ) -> Result<PublicFileLinkRequestBuilder, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        PublicFileLinkRequestBuilder::for_file(&self, file_like)
    }

    /// Returns the public download link for a public file link
    pub async fn get_public_download_link_for_file(
        &self,
        link: &pcloud_model::PublicFileLink,
    ) -> Result<pcloud_model::DownloadLink, Box<dyn std::error::Error>> {
        PublicFileDownloadRequestBuilder::for_public_file(self, link.code.clone().unwrap().as_str())
            .get()
            .await
    }

    /// Returns the download link for a file.  Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub async fn get_download_link_for_file<'a, T: TryInto<PCloudFile>>(
        &self,
        file_like: T,
    ) -> Result<pcloud_model::DownloadLink, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        FileDownloadRequestBuilder::for_file(self, file_like)?
            .get()
            .await
    }

    /// Get user info
    pub async fn get_user_info(&self) -> Result<UserInfo, Box<dyn std::error::Error>> {
        let url = format!("{}/userinfo", self.api_host);
        let mut r = self.client.get(url);

        r = self.add_token(r);

        debug!("Requesting user info");
        let userinfo = r.send().await?.json::<UserInfo>().await?.assert_ok()?;

        Ok(userinfo)
    }

    /// Downloads a DownloadLink
    pub async fn download_link(
        &self,
        link: &pcloud_model::DownloadLink,
    ) -> Result<Response, Box<dyn std::error::Error>> {
        if link.hosts.len() > 0 && link.path.is_some() {
            let url = format!(
                "https://{}{}",
                link.hosts.get(0).unwrap(),
                link.path.as_ref().unwrap()
            );

            debug!(
                "Downloading file link https://{}{}",
                link.hosts.get(0).unwrap(),
                link.path.as_ref().unwrap()
            );

            let mut r = self.client.get(url);

            r = self.add_token(r);

            let resp = r.send().await?;

            Ok(resp)
        } else {
            Err(PCloudResult::ProvideURL)?
        }
    }

    /// Fetches the download link and directly downloads the file.  Accepts either a file id (u64), a file path (String) or any other pCloud object describing a file (like Metadata)
    pub async fn download_file<'a, T: TryInto<PCloudFile>>(
        &self,
        file_like: T,
    ) -> Result<Response, Box<dyn 'a + std::error::Error>>
    where
        T::Error: 'a + std::error::Error,
    {
        let link = self.get_download_link_for_file(file_like).await?;
        self.download_link(&link).await
    }
}
