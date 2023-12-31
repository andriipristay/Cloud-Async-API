use std::fmt::Display;

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::*;

/// # ERRORS
/// There are number of cases when you request can't be processed as is and an error will be returned. Error codes are always 4 digit. They can be grouped into few categories depending on the type of error occurred.
/// @see https://docs.pcloud.com/errors/
///
/// | Code  |	Description |
/// | ----  | ------------- |
/// | 1xxx	| These errors are reserved for cases when the API client misbehaved. Most of the time it means that required parameters were not provided, text was provided when a number was expected, or one of several valid values was expected, but the input was something else. Also, trying to call a method that requires login without providing any login credentials is a 1xxx error, while providing bad credentials is not. Well behaved applications should never receive this type of error, regardless of user actions. It is advisable to find a way to send the error and the error message to the application developer. |
/// | 19xx	| This is sub-type of 1xxx errors. It may be the case that the application is misbehaving or it could be a synchronization error - e.g. you are trying to monitor the progress of an upload that the server knows nothing about. It might be the case that the application has passed a wrong or not existing hash or it could be that the upload request is still in transit and the API server is yet to start processing it. If you are sure that you have passed the correct parameters, it is safe to retry the request later. |
/// | 2xxx	| The user is trying to preform invalid operation or is providing bad data. Example errors are bad filename supplied, file not found or folder already exists. While a part of these can be prevented in the application (notably can not delete root folder), given the multi-user and multi client environment, files that were here just a moment ago may disappear. Generally these types of errors can be displayed directly to the user. However, it is preferable for the applications to actually understand the error codes instead of blindly displaying them. Of course, in some cases this errors can be the application's fault - e.g. the user wanted to open a file, but the application provided incorrect folderid. Keep in mind that user here is a quite abstract concept. If your applications is a filesystem, your users are not the end users, but end users' applications. |
/// | 3xxx	| These are rare errors when something can not be done and is unlikely that retrying will give any better results. One example of error of this type is trying to create a thumbnail from text file renamed to mypicture.jpg. It can't be classified as 1xxx error as the application did nothing wrong - it received thumb: true and decided to create thumbnail. The user probably didn't do anything wrong either (apart from renaming text file to mypicture.jpg, but it was probably the application that decided to display the thumbnail). These errors should be ignored if the unsuccessful action is not explicitly requested by user (fall back for failing to display a thumbnail would be to simply display an icon instead) and if the action was indeed requested by user, it should be reported that the file is bad. |
/// | 4xxx	| Should generally be very rare. They are reserved for cases when server is not willing to process you request. This generally means that the API server is rate limiting you because of too many requests or login tries. It should be possible to retry the request at a later stage. |
/// | 5xxx	| Errors of this type are the ones that we work very hard to never happen. Nevertheless they are still possible. These type of errors generally mean that we can not satisfy the request at this time (e.g. a server is unavailable) but it is very likely that the API server will be able to satisfy the request at a later stage. |
/// | 6xxx	| These are not real errors, but legitimate non-error answers. They are used by conditional methods mostly to signal some action not required state |
/// | 7xxx	| These errors generally represent error condition for which neither the implementation that accesses the API nor it's user are responsible. These errors should be expected when a method is indicated to return one of those and should be presented to the user more like a normal condition, rather than you got an error, the sky is falling down. Typical 7xxx error is for example when somebody has deleted his public link and the user is trying to access it. |
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone)]
#[repr(u16)]
pub enum PCloudResult {
    /// No Error    
    Ok = 0,
    LogInRequired = 1000,
    NoFullPathOrNameOrFolderIdProvided = 1001,
    NoFullPathOrFolderIdProvided = 1002,
    NoFileIdOrPathProvided = 1004,
    InvalidFileDescriptor = 1007,
    DateTimeFormatNotUnderstood = 1013,
    NoFullToPathOrToNameAndToFolderIdProvided = 1016,
    InvalidFolderId = 1017,
    InvalidFileId = 1018,
    ProvidedAtLeastToPathOrToFolderIdOrToName = 1037,
    ProvideURL = 1040,
    LoginFailed = 2000,
    InvalidFileOrFolderName = 2001,
    ComponentOfTheParentDirectoryDoesNotExist = 2002,
    AccessDenied = 2003,
    DirectoryDoesNotExist = 2005,
    FolderIsNotEmpty = 2006,
    CanNotDeleteRootFolder = 2007,
    UserOverQuota = 2008,
    FileNotFound = 2009,
    InvalidPath = 2010,
    PleaseVerifyYourMailAddressToPerformThisAction = 2014,
    CannotPlaceASharedFolderIntoAnotherSharedFolder = 2023,
    YouCanOnlyShareYourOwnFilesOrFolders = 2026,
    ActiveSharesOrShareRequestsForThisFolder = 2028,
    ConnectionBroken = 2041,
    CannotRenameTheRootFolder = 2042,
    CannotMoveAFolderToASubfolderOfItself = 2043,
    TooManyLogins = 4000,
    InternalError = 5000,
    InternalUploadError = 5001,
    WriteError = 5003,
}

/// Necessary to implement Error trait
impl Display for PCloudResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PCloudResult::Ok => write!(f, "Everything ok - no error"),
            PCloudResult::NoFullPathOrNameOrFolderIdProvided => {
                write!(f, "No full path or name/folderid provided.")
            }
            PCloudResult::LogInRequired => write!(f, "Log in required"),
            PCloudResult::NoFullPathOrFolderIdProvided => {
                write!(f, "No full path or folder id provided.")
            }
            PCloudResult::NoFileIdOrPathProvided => write!(f, "No file id or file path provided"),
            PCloudResult::DateTimeFormatNotUnderstood => {
                write!(f, "Date time format not understood")
            }

            PCloudResult::ProvideURL => write!(f, "Provide url"),
            PCloudResult::LoginFailed => write!(f, "Log in failed"),
            PCloudResult::InvalidFileOrFolderName => write!(f, "Invalid file or folder name"),
            PCloudResult::ComponentOfTheParentDirectoryDoesNotExist => {
                write!(f, "A component of the parent directory does not exist")
            }
            PCloudResult::AccessDenied => write!(f, "Access denied"),
            PCloudResult::DirectoryDoesNotExist => write!(f, "Directory does not exist"),
            PCloudResult::UserOverQuota => write!(f, "User over quota"),
            PCloudResult::FileNotFound => write!(f, "File not found"),
            PCloudResult::InvalidPath => write!(f, "Invalid path"),
            PCloudResult::PleaseVerifyYourMailAddressToPerformThisAction => {
                write!(f, "Please verify your mail address to perform this action")
            }
            PCloudResult::YouCanOnlyShareYourOwnFilesOrFolders => {
                write!(f, "You can only share your own files or folders")
            }
            PCloudResult::ConnectionBroken => write!(f, "Connection broken"),
            PCloudResult::TooManyLogins => write!(f, "Too many logins"),
            PCloudResult::InternalError => write!(f, "Internal error"),
            PCloudResult::InternalUploadError => write!(f, "Internal upload error"),
            PCloudResult::FolderIsNotEmpty => write!(f, "Folder is not empty"),
            PCloudResult::CanNotDeleteRootFolder => write!(f, "Cannot delete the root folder."),
            PCloudResult::ActiveSharesOrShareRequestsForThisFolder => write!(
                f,
                "There are active shares or sharerequests for this folder."
            ),
            PCloudResult::CannotRenameTheRootFolder => write!(f, "Cannot rename the root folder."),
            PCloudResult::CannotMoveAFolderToASubfolderOfItself => {
                write!(f, "Cannot move a folder to a subfolder of itself.")
            }
            PCloudResult::CannotPlaceASharedFolderIntoAnotherSharedFolder => write!(
                f,
                "You are trying to place shared folder into another shared folder."
            ),
            PCloudResult::ProvidedAtLeastToPathOrToFolderIdOrToName => write!(
                f,
                "Please provide at least one of 'topath', 'tofolderid' or 'toname'."
            ),
            PCloudResult::InvalidFileId => write!(f, "Invalid 'fileid' provided."),
            PCloudResult::InvalidFolderId => write!(f, "Invalid 'folderid' provided."),
            PCloudResult::NoFullToPathOrToNameAndToFolderIdProvided => {
                write!(f, "No full topath or toname/tofolderid provided.")
            }
            PCloudResult::WriteError => write!(f, "Write error. Try reopening the file."),
            PCloudResult::InvalidFileDescriptor => write!(f, "Invalid or closed file descriptor."),
        }
    }
}
/// PCloudResult implements the Error trait
impl std::error::Error for PCloudResult {}

/// Category of the file
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum FileCategory {
    Uncategorized = 0,
    Image = 1,
    Video = 2,
    Audio = 3,
    Document = 4,
    Archive = 5,
}

/// Icon of the file / folder
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileIcon {
    Document,
    Database,
    Archive,
    Web,
    Gis,
    Spreadsheet,
    Font,
    Presentation,
    Image,
    Diskimage,
    Package,
    Executable,
    Audio,
    Video,
    File,
    Folder,
}

/// Implemented by all structs having a PCloud Result
pub trait WithPCloudResult {
    /// Returns the result
    fn get_result(&self) -> &PCloudResult;

    /// Checks if the operation was 'Ok', if not Error is returned
    fn assert_ok(self) -> Result<Self, PCloudResult>
    where
        Self: Sized,
    {
        match self.get_result() {
            PCloudResult::Ok => Ok(self),
            x => Err(x.clone()),
        }
    }
}

/// Result of the `getpublinkdownload` or `getfilelink` calls
/// see https://docs.pcloud.com/methods/public_links/getpublinkdownload.html
/// see https://docs.pcloud.com/methods/streaming/getfilelink.html
#[derive(Serialize, Deserialize, Debug)]
pub struct DownloadLink {
    pub result: PCloudResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "pcloud_option_date_format")]
    pub expires: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub hosts: Vec<String>,
}

impl DownloadLink {
    /// Converts the given DownloadLink into a full download url for the file. There is no authentication necessary to download the file!
    pub fn into_url(&self) -> Option<String> {
        if self.result == PCloudResult::Ok && self.hosts.len() > 0 && self.path.is_some() {
            let url = format!(
                "https://{}{}",
                self.hosts.get(0).unwrap(),
                self.path.as_ref().unwrap()
            );
            Some(url)
        } else {
            None
        }
    }
}

impl WithPCloudResult for DownloadLink {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of the `getfilepublink` call
/// see https://docs.pcloud.com/methods/public_links/getfilepublink.html
#[derive(Serialize, Deserialize, Debug)]
pub struct PublicFileLink {
    pub result: PCloudResult,
    /// ID that can be used to delete/modify this public link
    pub linkid: Option<u64>,
    /// link's code that can be used to retrieve the public link contents  (with showpublink/getpublinkdownload)
    pub code: Option<String>,
    /// Full link
    pub link: Option<String>,
    ///  short code that can also be passed to showpublink/getpublinkdownload
    pub shortcode: Option<String>,
    /// a full https link to pc.cd domain with shortcode appended
    pub shortlink: Option<String>,
    /// Metadata of the target file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    /// date/time when the share request is sent
    #[serde(with = "pcloud_option_date_format")]
    pub created: Option<DateTime<Utc>>,
    /// date/time when the share request was modified
    #[serde(with = "pcloud_option_date_format")]
    pub modified: Option<DateTime<Utc>>,
    pub downloadenabled: Option<bool>,
    pub downloads: Option<u64>,
}

impl WithPCloudResult for PublicFileLink {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of the `diff` call
/// see https://docs.pcloud.com/methods/general/diff.html
#[derive(Serialize, Deserialize, Debug)]
pub struct Diff {
    /// Last diff id listed
    pub diffid: u64,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub entries: Vec<DiffEntry>,
}

/// Result of the `getfilehistory` call
/// @see https://docs.pcloud.com/methods/general/getfilehistory.html
#[derive(Serialize, Deserialize, Debug)]
pub struct FileHistory {
    /// Result of the operation
    pub result: PCloudResult,
    /// File history
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub entries: Vec<DiffEntry>,
}

impl WithPCloudResult for FileHistory {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// On success in the reply there will be entries array of objects and diffid. Set your current diffid to the provided diffid after you process all events, during processing set your state to the diffid of the event preferably in a single transaction with the event itself.
#[derive(Serialize, Deserialize, Debug)]
pub struct DiffEntry {
    /// Timestamp of the vent
    #[serde(with = "pcloud_date_format")]
    pub time: DateTime<Utc>,
    /// ID of the event
    pub diffid: u64,
    /// Type of the event
    pub event: DiffEvent,
    /// File metadata of file / folder targeted by the event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    /// Share metdata of the file / folder targeted by the event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<Share>,
}

/// Event can be one of:
/// see https://docs.pcloud.com/structures/event.html
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiffEvent {
    /// client should reset it's state to empty root directory
    Reset,
    /// folder is created, metadata is provided
    CreateFolder,
    /// folder is deleted, metadata is provided
    DeleteFolder,
    /// folder is modified, metadata is provided
    ModifyFolder,
    /// file is created, metadata is provided
    CreateFile,
    /// file data is modified, metadata is provided (normally modifytime, size and hash are changed)
    ModifyFile,
    /// file is deleted, metadata is provided
    DeleteFile,
    /// incoming share, share is provided
    RequestShareIn,
    /// you have accepted a share request (potentially on another device), useful to decrement the counter of pending requests. share is provided.It is guaranteed that you receive createfolder for the folderid (and all the contents of the folder) of the share before you receive acceptedshare, so it is safe to assume that you will be able to find folderid in the local state.
    AcceptedShareIn,
    /// you have declined a share request, share is provided (this is delivered to the declining user, not to the sending one)<
    DeclinedShareIn,
    /// same as above, but delivered to the user that is sharing the folder.
    DeclinedShareOut,
    /// the sender of a share request cancelled the share request
    CancelledShareIn,
    /// your incoming share is removed (either by you or the other user)
    RemovedShareIn,
    /// your incoming share in is modified (permissions changed)
    ModifiedShareIn,
    /// user's information is modified, includes userinfo object
    ModifyUserInfo,
}

/// For shares, a "share" object is provided with keys
/// https://docs.pcloud.com/structures/share.html
#[derive(Serialize, Deserialize, Debug)]
pub struct Share {
    pub folderid: u64,
    ///  id of the sharerequest, can be used to accept request, not available in removeshare and modifiedshare
    pub sharerequestid: Option<u64>,
    /// shareid of the share, only available in acceptedshare* and removeshare
    pub shareid: Option<u64>,
    /// name of the share, normally that is the name of the directory the user is sharing, not available in removeshare* and modifiedshare
    pub sharename: Option<String>,
    /// date/time when the share request is sent, not available in removeshare* and modifiedshare
    #[serde(with = "pcloud_option_date_format")]
    pub created: Option<DateTime<Utc>>,
    /// date/time when the share request expires, not available in removeshare* and modifiedshare
    #[serde(with = "pcloud_option_date_format")]
    pub expires: Option<DateTime<Utc>>,
    /// flag that you are granded read permissions, not available in removeshare
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canread: Option<bool>,
    /// flag that you are granded modify permissions, not available in removeshare
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canmodify: Option<bool>,
    /// flag that you are granded delete permissions, not available in removeshare
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candelete: Option<bool>,
    /// flag that you are granded create permissions, not available in removeshare
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancreate: Option<bool>,
    /// optional message provided by the user offering share (may not be provided), not available in removeshare* and modifiedshare*
    pub message: Option<String>,
}

/// The metadata for a file or folder normally consists of:
/// see https://docs.pcloud.com/structures/metadata.html
#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    // is the folderid of the folder the object resides in
    pub parentfolderid: Option<u64>,
    //  is it a folder(true) or file(false)
    pub isfolder: bool,
    /// is the object owned by the user if ismine is false than four other bool fields are provided: canread, canmodify, candelete, cancreate (cancreate - only for folders). These are user's permissions for this object Also, when ismine is false, userid is provided with the id of the owner of the file/folder.
    pub ismine: bool,
    /// flag that you are granded read permissions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canread: Option<bool>,
    ///  flag that you are granded modify permissions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canmodify: Option<bool>,
    /// flag that you are granded delete permissions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candelete: Option<bool>,
    /// flag that you are granded create permissions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancreate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub userid: Option<u64>,
    ///  is the object shared with other users
    pub isshared: bool,
    /// the name of file or folder
    pub name: String,
    ///  unique string id. For folders this is folderid prepended with letter d and for files it is the fileid with f in front.
    pub id: String,
    /// for folders the folderid of the folder
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folderid: Option<u64>,
    /// for files file's fileid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fileid: Option<u64>,
    ///  It is possible that as a result of renamefile operation a file with the same name gets deleted (e.g. file old.txt is renamed to new.txt when new.txt already exists in this folder). In these cases deletedfileid is set to fileid of the deleted file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletefileid: Option<String>,
    /// creation date of the object
    #[serde(with = "pcloud_date_format")]
    pub created: DateTime<Utc>,
    ///  modification date of the object
    #[serde(with = "pcloud_date_format")]
    pub modified: DateTime<Utc>,
    /// name of the icon to display
    pub icon: Option<FileIcon>,
    /// category of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<FileCategory>,
    /// true if thumbs can be created from the object
    pub thumb: bool,
    // size in bytes, present only for files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    ///  content-type of the file, present only for files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contenttype: Option<String>,
    /// 64 bit integer representing hash of the contents of the file can be used to determine if two files are the same or to monitor file contents for changes. Present only for files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<u64>,
    /// array of metadata objects representing contents of the directory
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub contents: Vec<Metadata>,
    /// isdeleted is never false, it is present only for deleted objects, only when deleted objects are requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isdeleted: Option<bool>,
    /// Full path might be provided in some cases. If you work with paths and request folders by path, it will be provided. Recursive listings do not have path provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Optional for images / videos: width of the image in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u64>,
    /// Optional for images / videos: height of the image in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u64>,
    /// Optional for audio files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    /// Optional for audio files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    /// Optional for audio files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional for audio files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    /// Optional for audio files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trackno: Option<String>,
    /// Optional for video files: duration of the video in seconds (floating point number sent as string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    /// Optional for video files: frames per second rate of the video (floating point number sent as string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<String>,
    /// Optional for video files: codec used for encoding of the video
    #[serde(skip_serializing_if = "Option::is_none")]
    pub videocodec: Option<String>,
    /// Optional for video files: codec used for encoding of the audio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audiocodec: Option<String>,
    /// Optional for video files: bitrate of the video in kilobits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub videobitrate: Option<u32>,
    /// Optional for video files: bitrate of the audio in kilobits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audiobitrate: Option<u32>,
    /// Optional for video files: sampling rate of the audio in Hz
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audiosamplerate: Option<u32>,
    /// Optional for video files:  indicates that video should be rotated (0, 90, 180 or 270) degrees when playing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<u16>,
}

/// Result of the `getapiserver`request
#[derive(Serialize, Deserialize, Debug)]
pub struct ApiServers {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
    /// API endpoints for the binary API (first entry is the best choice)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub binapi: Vec<String>,
    /// API endpoints for the rest API (first entry is the best choice)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub api: Vec<String>,
}

impl WithPCloudResult for ApiServers {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of fetching metadata of files or folders
/// see https://docs.pcloud.com/methods/file/stat.html
/// see https://docs.pcloud.com/methods/folder/listfolder.html
#[derive(Serialize, Deserialize, Debug)]
pub struct FileOrFolderStat {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
    /// Metadata of the targeted file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

impl WithPCloudResult for FileOrFolderStat {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileRevision {
    /// id of the revision
    pub revisionid: u64,
    ///  filesize of the given revision of the file
    pub size: u64,
    /// file contents hash (same as in metadata)
    pub hash: u64,
    /// date/time at which the revision was created
    #[serde(with = "pcloud_date_format")]
    pub created: DateTime<Utc>,
}

/// Result of the listrevisions call
/// see https://docs.pcloud.com/methods/revisions/listrevisions.html
#[derive(Serialize, Deserialize, Debug)]
pub struct RevisionList {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
    /// Metadata of the targeted file
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<Metadata>,
    ///  Lists the revisions as array
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub revisions: Vec<FileRevision>,
}

impl WithPCloudResult for RevisionList {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of the deletefolderrecursive operation
/// see https://docs.pcloud.com/methods/folder/deletefolderrecursive.html
#[derive(Serialize, Deserialize, Debug)]
pub struct FolderRecursivlyDeleted {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
    /// the number of deleted files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletedfiles: Option<u64>,
    /// number of deleted folders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletedfolders: Option<u64>,
}

impl WithPCloudResult for FolderRecursivlyDeleted {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of calculating file checksums
/// see https://docs.pcloud.com/methods/file/checksumfile.html
#[derive(Serialize, Deserialize, Debug)]
pub struct FileChecksums {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
    /// Metdata of the target file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    // SHA-1 checksum. Always present
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    /// MD5 checksum, is returned only from US API servers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
    /// SHA-256 checksum is returned in Europe only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

impl WithPCloudResult for FileChecksums {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of fetching user metadata
/// see https://docs.pcloud.com/methods/general/userinfo.html
#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
    /// Authentication token (only present if getauth query parameter was set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    // Unique id of the user
    pub userid: Option<u64>,
    /// email address of the user
    pub email: Option<String>,
    /// true if the user had verified it's email
    pub emailverified: Option<bool>,
    /// when the user was registered
    #[serde(with = "pcloud_option_date_format")]
    pub registered: Option<DateTime<Utc>>,
    /// 2-3 characters lowercase languageid
    pub language: Option<String>,
    ///  true if the user is premium
    pub premium: Option<bool>,
    ///  quota in bytes, so quite big numbers
    pub usedquota: Option<u64>,
    /// quota in bytes
    pub quota: Option<u64>,
}

impl WithPCloudResult for UserInfo {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of a file upload operation
/// see https://docs.pcloud.com/methods/file/uploadfile.html
#[derive(Serialize, Deserialize, Debug)]
pub struct UploadedFile {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fileids: Vec<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub metadata: Vec<Metadata>,
}

impl WithPCloudResult for UploadedFile {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of log out
/// see https://docs.pcloud.com/methods/auth/logout.html
#[derive(Serialize, Deserialize, Debug)]
pub struct LogoutResponse {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
    /// Authentication token successfully deleted?
    pub auth_deleted: Option<bool>,
}

impl WithPCloudResult for LogoutResponse {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of opening a file
/// see https://docs.pcloud.com/methods/fileops/file_open.html
#[derive(Serialize, Deserialize, Debug)]
pub struct FileOpenResponse {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
    /// File descriptor
    pub fd: u64,
    /// File id
    pub fileid: u64,
}

impl WithPCloudResult for FileOpenResponse {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of closing a file
/// see https://docs.pcloud.com/methods/fileops/file_close.html
#[derive(Serialize, Deserialize, Debug)]
pub struct FileCloseResponse {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
}

impl WithPCloudResult for FileCloseResponse {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Result of writing a file
/// see https://docs.pcloud.com/methods/fileops/file_write.html
#[derive(Serialize, Deserialize, Debug)]
pub struct FileWriteResponse {
    /// Result of the operation, must be Ok for further values to be present
    pub result: PCloudResult,
    /// number of bytes written
    pub bytes: Option<u64>,
}

impl WithPCloudResult for FileWriteResponse {
    fn get_result(&self) -> &PCloudResult {
        &self.result
    }
}

/// Get the progress in process of zipping file in the user's filesystem.
/// see https://docs.pcloud.com/methods/archiving/savezipprogress.html
#[derive(Serialize, Deserialize, Debug)]
pub struct SaveZipProgressResponse {
    /// count of the already zipped files.
    pub files: u64,
    ///  total count of files to be zipped.
    pub totalfiles: u64,
    /// size of the already zipped files.
    pub bytes: u64,
    /// total size of the files to be zipped.
    pub totalbytes: u64,
}

/// Converts a DateTime for pCloud URLs
pub fn format_date_time_for_pcloud<Tz>(datetime: &DateTime<Tz>) -> String
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    let format = "%a, %d %b %Y %H:%M:%S %z";
    format!("{}", datetime.format(format))
}

/// pCloud Date format for serializing / deserializing
mod pcloud_date_format {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};
    const FORMAT: &'static str = "%a, %d %b %Y %H:%M:%S %z";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}

/// pCloud Date format for serializing / deserializing optional values
mod pcloud_option_date_format {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};
    const FORMAT: &'static str = "%a, %d %b %Y %H:%M:%S %z";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(inp: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match inp {
            Some(date) => {
                let s = format!("{}", date.format(FORMAT));
                serializer.serialize_str(&s)
            }
            None => serializer.serialize_none(),
        }
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inp = String::deserialize(deserializer);

        match inp {
            Ok(s) => {
                let conv = Utc
                    .datetime_from_str(&s, FORMAT)
                    .map_err(serde::de::Error::custom);

                match conv {
                    Ok(v) => Ok(Some(v)),
                    Err(e) => Err(e),
                }
            }
            Err(_) => Ok(None),
        }
    }
}
