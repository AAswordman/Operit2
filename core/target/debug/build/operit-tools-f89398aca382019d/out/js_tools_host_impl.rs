// Generated from canonical Rust Tools traits. Do not edit.

impl FilesHost for AIToolHandler {
    /// Executes the canonical `Files.list` Tools binding.
    fn list (& self , path : String) -> JsFuture < DirectoryListingData > {
        invoke_generated(
            self,
            BuiltinToolName::ListFiles,
            "Files",
            "list",
            vec![generated_argument("path", path)],
        )
    }

    /// Executes the canonical `Files.read` Tools binding.
    fn read_overload_1 (& self , path : String) -> JsFuture < FileContentData > {
        invoke_generated(
            self,
            BuiltinToolName::ReadFileFull,
            "Files",
            "read",
            vec![generated_argument("path", path)],
        )
    }

    /// Executes the canonical `Files.read` Tools binding.
    fn read_overload_2 (& self , options : FilesReadFileOptions) -> JsFuture < FileContentData > {
        invoke_generated(
            self,
            BuiltinToolName::ReadFileFull,
            "Files",
            "read",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Files.readPart` Tools binding.
    fn readPart (& self , path : String , startLine : Option < f64 > , endLine : Option < f64 > ,) -> JsFuture < FilePartContentData > {
        invoke_generated(
            self,
            BuiltinToolName::ReadFilePart,
            "Files",
            "readPart",
            vec![generated_argument("path", path), generated_argument("startLine", startLine), generated_argument("endLine", endLine)],
        )
    }

    /// Executes the canonical `Files.write` Tools binding.
    fn write (& self , path : String , content : String , append : Option < bool > ,) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::WriteFile,
            "Files",
            "write",
            vec![generated_argument("path", path), generated_argument("content", content), generated_argument("append", append)],
        )
    }

    /// Executes the canonical `Files.writeBinary` Tools binding.
    fn writeBinary (& self , path : String , base64Content : String) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::WriteFileBinary,
            "Files",
            "writeBinary",
            vec![generated_argument("path", path), generated_argument("base64Content", base64Content)],
        )
    }

    /// Executes the canonical `Files.readBinary` Tools binding.
    fn readBinary (& self , path : String) -> JsFuture < BinaryFileContentData > {
        invoke_generated(
            self,
            BuiltinToolName::ReadFileBinary,
            "Files",
            "readBinary",
            vec![generated_argument("path", path)],
        )
    }

    /// Executes the canonical `Files.deleteFile` Tools binding.
    fn deleteFile (& self , path : String , recursive : Option < bool >) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::DeleteFile,
            "Files",
            "deleteFile",
            vec![generated_argument("path", path), generated_argument("recursive", recursive)],
        )
    }

    /// Executes the canonical `Files.exists` Tools binding.
    fn exists (& self , path : String) -> JsFuture < FileExistsData > {
        invoke_generated(
            self,
            BuiltinToolName::FileExists,
            "Files",
            "exists",
            vec![generated_argument("path", path)],
        )
    }

    /// Executes the canonical `Files.move` Tools binding.
    fn r#move (& self , source : String , destination : String) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::MoveFile,
            "Files",
            "move",
            vec![generated_argument("source", source), generated_argument("destination", destination)],
        )
    }

    /// Executes the canonical `Files.copy` Tools binding.
    fn copy (& self , source : String , destination : String , recursive : Option < bool > ,) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::CopyFile,
            "Files",
            "copy",
            vec![generated_argument("source", source), generated_argument("destination", destination), generated_argument("recursive", recursive)],
        )
    }

    /// Executes the canonical `Files.mkdir` Tools binding.
    fn mkdir (& self , path : String , create_parents : Option < bool >) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::MakeDirectory,
            "Files",
            "mkdir",
            vec![generated_argument("path", path), generated_argument("create_parents", create_parents)],
        )
    }

    /// Executes the canonical `Files.find` Tools binding.
    fn find (& self , path : String , pattern : String , options : Option < BTreeMap < String , serde_json :: Value > > ,) -> JsFuture < FindFilesResultData > {
        invoke_generated(
            self,
            BuiltinToolName::FindFiles,
            "Files",
            "find",
            vec![generated_argument("path", path), generated_argument("pattern", pattern), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Files.grep` Tools binding.
    fn grep (& self , path : String , pattern : String , options : Option < FilesHostGrepOptions > ,) -> JsFuture < GrepResultData > {
        invoke_generated(
            self,
            BuiltinToolName::GrepCode,
            "Files",
            "grep",
            vec![generated_argument("path", path), generated_argument("pattern", pattern), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Files.grepContext` Tools binding.
    fn grepContext (& self , path : String , intent : String , options : Option < FilesHostGrepContextOptions > ,) -> JsFuture < GrepResultData > {
        invoke_generated(
            self,
            BuiltinToolName::GrepContext,
            "Files",
            "grepContext",
            vec![generated_argument("path", path), generated_argument("intent", intent), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Files.info` Tools binding.
    fn info (& self , path : String) -> JsFuture < FileInfoData > {
        invoke_generated(
            self,
            BuiltinToolName::FileInfo,
            "Files",
            "info",
            vec![generated_argument("path", path)],
        )
    }

    /// Executes the canonical `Files.apply` Tools binding.
    fn apply (& self , path : String , r#type : ApplyFileType , old : Option < String > , newContent : Option < String > ,) -> JsFuture < FileApplyResultData > {
        invoke_generated(
            self,
            BuiltinToolName::ApplyFile,
            "Files",
            "apply",
            vec![generated_argument("path", path), generated_argument("r#type", r#type), generated_argument("old", old), generated_argument("newContent", newContent)],
        )
    }

    /// Executes the canonical `Files.create` Tools binding.
    fn create (& self , path : String , newContent : String) -> JsFuture < FileApplyResultData > {
        invoke_generated(
            self,
            BuiltinToolName::CreateFile,
            "Files",
            "create",
            vec![generated_argument("path", path), generated_argument("newContent", newContent)],
        )
    }

    /// Executes the canonical `Files.edit` Tools binding.
    fn edit (& self , path : String , oldContent : String , newContent : String ,) -> JsFuture < FileApplyResultData > {
        invoke_generated(
            self,
            BuiltinToolName::EditFile,
            "Files",
            "edit",
            vec![generated_argument("path", path), generated_argument("oldContent", oldContent), generated_argument("newContent", newContent)],
        )
    }

    /// Executes the canonical `Files.zip` Tools binding.
    fn zip (& self , source : String , destination : String , include_root_directory : Option < bool > ,) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::ZipFiles,
            "Files",
            "zip",
            vec![generated_argument("source", source), generated_argument("destination", destination), generated_argument("include_root_directory", include_root_directory)],
        )
    }

    /// Executes the canonical `Files.unzip` Tools binding.
    fn unzip (& self , source : String , destination : String) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::UnzipFiles,
            "Files",
            "unzip",
            vec![generated_argument("source", source), generated_argument("destination", destination)],
        )
    }

    /// Executes the canonical `Files.open` Tools binding.
    fn open (& self , path : String) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::OpenFile,
            "Files",
            "open",
            vec![generated_argument("path", path)],
        )
    }

    /// Executes the canonical `Files.share` Tools binding.
    fn share (& self , path : String , title : Option < String >) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::ShareFile,
            "Files",
            "share",
            vec![generated_argument("path", path), generated_argument("title", title)],
        )
    }

    /// Executes the canonical `Files.download` Tools binding.
    fn download_overload_1 (& self , url : String , destination : String , headers : Option < BTreeMap < String , String > > ,) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::DownloadFile,
            "Files",
            "download",
            vec![generated_argument("url", url), generated_argument("destination", destination), generated_argument("headers", headers)],
        )
    }

    /// Executes the canonical `Files.download` Tools binding.
    fn download_overload_2 (& self , options : FilesHostDownloadOptions) -> JsFuture < FileOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::DownloadFile,
            "Files",
            "download",
            vec![generated_argument("options", options)],
        )
    }

}

impl NetHost for AIToolHandler {
    /// Executes the canonical `Net.httpGet` Tools binding.
    fn httpGet (& self , url : String , ignore_ssl : Option < bool >) -> JsFuture < HttpResponseData > {
        invoke_generated(
            self,
            BuiltinToolName::HttpRequest,
            "Net",
            "httpGet",
            vec![generated_argument("url", url), generated_argument("ignore_ssl", ignore_ssl)],
        )
    }

    /// Executes the canonical `Net.httpPost` Tools binding.
    fn httpPost (& self , url : String , body : NetHostHttpPostBody , ignore_ssl : Option < bool > ,) -> JsFuture < HttpResponseData > {
        invoke_generated(
            self,
            BuiltinToolName::HttpRequest,
            "Net",
            "httpPost",
            vec![generated_argument("url", url), generated_argument("body", body), generated_argument("ignore_ssl", ignore_ssl)],
        )
    }

    /// Executes the canonical `Net.visit` Tools binding.
    fn visit (& self , urlOrParams : NetHostVisitUrlOrParams) -> JsFuture < VisitWebResultData > {
        invoke_generated(
            self,
            BuiltinToolName::VisitWeb,
            "Net",
            "visit",
            vec![generated_argument("urlOrParams", urlOrParams)],
        )
    }

    /// Executes the canonical `Net.browserNavigate` Tools binding.
    fn browserNavigate (& self , urlOrOptions : NetHostBrowserNavigateUrlOrOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserNavigate,
            "Net",
            "browserNavigate",
            vec![generated_argument("urlOrOptions", urlOrOptions)],
        )
    }

    /// Executes the canonical `Net.browserNavigateBack` Tools binding.
    fn browserNavigateBack (& self , options : Option < BTreeMap < String , operit_plugin_sdk :: js_sdk :: JsNever > > ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserNavigateBack,
            "Net",
            "browserNavigateBack",
            vec![generated_empty_argument("options")],
        )
    }

    /// Executes the canonical `Net.browserClick` Tools binding.
    fn browserClick (& self , options : NetHostBrowserClickOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserClick,
            "Net",
            "browserClick",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserClose` Tools binding.
    fn browserClose (& self , options : Option < BTreeMap < String , operit_plugin_sdk :: js_sdk :: JsNever > >) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserClose,
            "Net",
            "browserClose",
            vec![generated_empty_argument("options")],
        )
    }

    /// Executes the canonical `Net.browserCloseAll` Tools binding.
    fn browserCloseAll (& self , options : Option < BTreeMap < String , operit_plugin_sdk :: js_sdk :: JsNever > > ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserCloseAll,
            "Net",
            "browserCloseAll",
            vec![generated_empty_argument("options")],
        )
    }

    /// Executes the canonical `Net.browserConsoleMessages` Tools binding.
    fn browserConsoleMessages (& self , options : Option < NetHostBrowserConsoleMessagesOptions > ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserConsoleMessages,
            "Net",
            "browserConsoleMessages",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserDrag` Tools binding.
    fn browserDrag (& self , options : NetHostBrowserDragOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserDrag,
            "Net",
            "browserDrag",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserEvaluate` Tools binding.
    fn browserEvaluate (& self , options : NetHostBrowserEvaluateOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserEvaluate,
            "Net",
            "browserEvaluate",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserFileUpload` Tools binding.
    fn browserFileUpload (& self , options : Option < NetHostBrowserFileUploadOptions > ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserFileUpload,
            "Net",
            "browserFileUpload",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserFillForm` Tools binding.
    fn browserFillForm (& self , options : NetHostBrowserFillFormOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserFillForm,
            "Net",
            "browserFillForm",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserHandleDialog` Tools binding.
    fn browserHandleDialog (& self , options : NetHostBrowserHandleDialogOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserHandleDialog,
            "Net",
            "browserHandleDialog",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserHover` Tools binding.
    fn browserHover (& self , options : NetHostBrowserHoverOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserHover,
            "Net",
            "browserHover",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserNetworkRequests` Tools binding.
    fn browserNetworkRequests (& self , options : Option < NetHostBrowserNetworkRequestsOptions > ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserNetworkRequests,
            "Net",
            "browserNetworkRequests",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserPressKey` Tools binding.
    fn browserPressKey (& self , keyOrOptions : NetHostBrowserPressKeyKeyOrOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserPressKey,
            "Net",
            "browserPressKey",
            vec![generated_argument("keyOrOptions", keyOrOptions)],
        )
    }

    /// Executes the canonical `Net.browserResize` Tools binding.
    fn browserResize (& self , options : NetHostBrowserResizeOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserResize,
            "Net",
            "browserResize",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserRunCode` Tools binding.
    fn browserRunCode (& self , options : NetHostBrowserRunCodeOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserRunCode,
            "Net",
            "browserRunCode",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserSelectOption` Tools binding.
    fn browserSelectOption (& self , options : NetHostBrowserSelectOptionOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserSelectOption,
            "Net",
            "browserSelectOption",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserSnapshot` Tools binding.
    fn browserSnapshot (& self , options : Option < NetHostBrowserSnapshotOptions >) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserSnapshot,
            "Net",
            "browserSnapshot",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserTakeScreenshot` Tools binding.
    fn browserTakeScreenshot (& self , options : NetHostBrowserTakeScreenshotOptions ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserTakeScreenshot,
            "Net",
            "browserTakeScreenshot",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserTabs` Tools binding.
    fn browserTabs (& self , options : NetHostBrowserTabsOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserTabs,
            "Net",
            "browserTabs",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserType` Tools binding.
    fn browserType (& self , options : NetHostBrowserTypeOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserType,
            "Net",
            "browserType",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.browserWaitFor` Tools binding.
    fn browserWaitFor (& self , options : NetHostBrowserWaitForOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BrowserWaitFor,
            "Net",
            "browserWaitFor",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.http` Tools binding.
    fn http (& self , options : NetHostHttpOptions) -> JsFuture < HttpResponseData > {
        invoke_generated(
            self,
            BuiltinToolName::HttpRequest,
            "Net",
            "http",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Net.uploadFile` Tools binding.
    fn uploadFile (& self , options : NetHostUploadFileOptions) -> JsFuture < HttpResponseData > {
        invoke_generated(
            self,
            BuiltinToolName::MultipartRequest,
            "Net",
            "uploadFile",
            vec![generated_argument("options", options)],
        )
    }

}

impl NetCookieManager for AIToolHandler {
    /// Executes the canonical `Net.cookies.get` Tools binding.
    fn get (& self , domain : String) -> JsFuture < HttpResponseData > {
        invoke_generated(
            self,
            BuiltinToolName::ManageCookies,
            "Net.cookies",
            "get",
            vec![generated_argument("domain", domain)],
        )
    }

    /// Executes the canonical `Net.cookies.set` Tools binding.
    fn set (& self , domain : String , cookies : NetCookieManagerSetCookies ,) -> JsFuture < HttpResponseData > {
        invoke_generated(
            self,
            BuiltinToolName::ManageCookies,
            "Net.cookies",
            "set",
            vec![generated_argument("domain", domain), generated_argument("cookies", cookies)],
        )
    }

    /// Executes the canonical `Net.cookies.clear` Tools binding.
    fn clear (& self , domain : Option < String >) -> JsFuture < HttpResponseData > {
        invoke_generated(
            self,
            BuiltinToolName::ManageCookies,
            "Net.cookies",
            "clear",
            vec![generated_argument("domain", domain)],
        )
    }

}

impl SystemHost for AIToolHandler {
    /// Executes the canonical `System.sleep` Tools binding.
    fn sleep (& self , milliseconds : SystemHostSleepMilliseconds) -> JsFuture < SleepResultData > {
        invoke_generated(
            self,
            BuiltinToolName::Sleep,
            "System",
            "sleep",
            vec![generated_argument("milliseconds", milliseconds)],
        )
    }

    /// Executes the canonical `System.getSetting` Tools binding.
    fn getSetting (& self , setting : String , namespace : Option < String >) -> JsFuture < SystemSettingData > {
        invoke_generated(
            self,
            BuiltinToolName::GetSystemSetting,
            "System",
            "getSetting",
            vec![generated_argument("setting", setting), generated_argument("namespace", namespace)],
        )
    }

    /// Executes the canonical `System.setSetting` Tools binding.
    fn setSetting (& self , setting : String , value : String , namespace : Option < String > ,) -> JsFuture < SystemSettingData > {
        invoke_generated(
            self,
            BuiltinToolName::ModifySystemSetting,
            "System",
            "setSetting",
            vec![generated_argument("setting", setting), generated_argument("value", value), generated_argument("namespace", namespace)],
        )
    }

    /// Executes the canonical `System.getDeviceInfo` Tools binding.
    fn getDeviceInfo (& self) -> JsFuture < DeviceInfoResultData > {
        invoke_generated(
            self,
            BuiltinToolName::DeviceInfo,
            "System",
            "getDeviceInfo",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `System.toast` Tools binding.
    fn toast (& self , message : String) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::Toast,
            "System",
            "toast",
            vec![generated_argument("message", message)],
        )
    }

    /// Executes the canonical `System.sendNotification` Tools binding.
    fn sendNotification (& self , message : String , title : Option < String >) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::SendNotification,
            "System",
            "sendNotification",
            vec![generated_argument("message", message), generated_argument("title", title)],
        )
    }

    /// Executes the canonical `System.usePackage` Tools binding.
    fn usePackage (& self , packageName : String) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::UsePackage,
            "System",
            "usePackage",
            vec![generated_argument("packageName", packageName)],
        )
    }

    /// Executes the canonical `System.installApp` Tools binding.
    fn installApp (& self , path : String) -> JsFuture < AppOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::InstallApp,
            "System",
            "installApp",
            vec![generated_argument("path", path)],
        )
    }

    /// Executes the canonical `System.uninstallApp` Tools binding.
    fn uninstallApp (& self , packageName : String) -> JsFuture < AppOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::UninstallApp,
            "System",
            "uninstallApp",
            vec![generated_argument("packageName", packageName)],
        )
    }

    /// Executes the canonical `System.stopApp` Tools binding.
    fn stopApp (& self , packageName : String) -> JsFuture < AppOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::StopApp,
            "System",
            "stopApp",
            vec![generated_argument("packageName", packageName)],
        )
    }

    /// Executes the canonical `System.listApps` Tools binding.
    fn listApps (& self , includeSystem : Option < bool >) -> JsFuture < AppListData > {
        invoke_generated(
            self,
            BuiltinToolName::ListInstalledApps,
            "System",
            "listApps",
            vec![generated_argument("includeSystem", includeSystem)],
        )
    }

    /// Executes the canonical `System.startApp` Tools binding.
    fn startApp (& self , packageName : String , activity : Option < String >) -> JsFuture < AppOperationData > {
        invoke_generated(
            self,
            BuiltinToolName::StartApp,
            "System",
            "startApp",
            vec![generated_argument("packageName", packageName), generated_argument("activity", activity)],
        )
    }

    /// Executes the canonical `System.getNotifications` Tools binding.
    fn getNotifications (& self , limit : Option < f64 > , includeOngoing : Option < bool > ,) -> JsFuture < NotificationData > {
        invoke_generated(
            self,
            BuiltinToolName::GetNotifications,
            "System",
            "getNotifications",
            vec![generated_argument("limit", limit), generated_argument("includeOngoing", includeOngoing)],
        )
    }

    /// Executes the canonical `System.getAppUsageTime` Tools binding.
    fn getAppUsageTime (& self , options : Option < SystemHostGetAppUsageTimeOptions > ,) -> JsFuture < AppUsageTimeResultData > {
        invoke_generated(
            self,
            BuiltinToolName::GetAppUsageTime,
            "System",
            "getAppUsageTime",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.getLocation` Tools binding.
    fn getLocation (& self , highAccuracy : Option < bool > , timeout : Option < f64 > , includeAddress : Option < bool > ,) -> JsFuture < LocationData > {
        invoke_generated(
            self,
            BuiltinToolName::GetDeviceLocation,
            "System",
            "getLocation",
            vec![generated_argument("highAccuracy", highAccuracy), generated_argument("timeout", timeout), generated_argument("includeAddress", includeAddress)],
        )
    }

}

impl SystemBluetoothHost for AIToolHandler {
    /// Executes the canonical `System.bluetooth.requestPermission` Tools binding.
    fn requestPermission (& self) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::RequestBluetoothPermission,
            "System.bluetooth",
            "requestPermission",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `System.bluetooth.getState` Tools binding.
    fn getState (& self) -> JsFuture < BluetoothStateData > {
        invoke_generated(
            self,
            BuiltinToolName::GetBluetoothState,
            "System.bluetooth",
            "getState",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `System.bluetooth.requestEnable` Tools binding.
    fn requestEnable (& self) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::RequestEnableBluetooth,
            "System.bluetooth",
            "requestEnable",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `System.bluetooth.listBondedDevices` Tools binding.
    fn listBondedDevices (& self) -> JsFuture < BluetoothBondedDevicesData > {
        invoke_generated(
            self,
            BuiltinToolName::ListBluetoothBondedDevices,
            "System.bluetooth",
            "listBondedDevices",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `System.bluetooth.scan` Tools binding.
    fn scan (& self , options : Option < SystemBluetoothHostScanOptions > ,) -> JsFuture < BluetoothScanResultData > {
        invoke_generated(
            self,
            BuiltinToolName::ScanBluetoothDevices,
            "System.bluetooth",
            "scan",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.connect` Tools binding.
    fn connect (& self , options : SystemBluetoothHostConnectOptions) -> JsFuture < BluetoothSessionData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothConnect,
            "System.bluetooth",
            "connect",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.listen` Tools binding.
    fn listen (& self , options : Option < SystemBluetoothHostListenOptions > ,) -> JsFuture < BluetoothSessionData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothListen,
            "System.bluetooth",
            "listen",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.accept` Tools binding.
    fn accept (& self , listenerSessionId : String , timeoutMs : Option < SystemBluetoothHostAcceptTimeoutMs > ,) -> JsFuture < BluetoothSessionData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothAccept,
            "System.bluetooth",
            "accept",
            vec![generated_argument("listenerSessionId", listenerSessionId), generated_argument("timeoutMs", timeoutMs)],
        )
    }

    /// Executes the canonical `System.bluetooth.send` Tools binding.
    fn send (& self , sessionId : String , options : SystemBluetoothHostSendOptions ,) -> JsFuture < BluetoothTransferData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothSend,
            "System.bluetooth",
            "send",
            vec![generated_argument("sessionId", sessionId), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.read` Tools binding.
    fn read (& self , sessionId : String , options : Option < SystemBluetoothHostReadOptions > ,) -> JsFuture < BluetoothReadData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothRead,
            "System.bluetooth",
            "read",
            vec![generated_argument("sessionId", sessionId), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.sendAndRead` Tools binding.
    fn sendAndRead (& self , sessionId : String , options : SystemBluetoothHostSendAndReadOptions ,) -> JsFuture < BluetoothReadData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothSendAndRead,
            "System.bluetooth",
            "sendAndRead",
            vec![generated_argument("sessionId", sessionId), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.close` Tools binding.
    fn close (& self , sessionId : String) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothClose,
            "System.bluetooth",
            "close",
            vec![generated_argument("sessionId", sessionId)],
        )
    }

}

impl SystemBluetoothBleHost for AIToolHandler {
    /// Executes the canonical `System.bluetooth.ble.connect` Tools binding.
    fn connect (& self , options : SystemBluetoothBleHostConnectOptions ,) -> JsFuture < BluetoothSessionData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothBleConnect,
            "System.bluetooth.ble",
            "connect",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.ble.discoverServices` Tools binding.
    fn discoverServices (& self , sessionId : String , timeoutMs : Option < SystemBluetoothBleHostDiscoverServicesTimeoutMs > ,) -> JsFuture < BluetoothBleServicesData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothBleDiscoverServices,
            "System.bluetooth.ble",
            "discoverServices",
            vec![generated_argument("sessionId", sessionId), generated_argument("timeoutMs", timeoutMs)],
        )
    }

    /// Executes the canonical `System.bluetooth.ble.readCharacteristic` Tools binding.
    fn readCharacteristic (& self , sessionId : String , options : SystemBluetoothBleHostReadCharacteristicOptions ,) -> JsFuture < BluetoothReadData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothBleReadCharacteristic,
            "System.bluetooth.ble",
            "readCharacteristic",
            vec![generated_argument("sessionId", sessionId), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.ble.writeCharacteristic` Tools binding.
    fn writeCharacteristic (& self , sessionId : String , options : SystemBluetoothBleHostWriteCharacteristicOptions ,) -> JsFuture < BluetoothTransferData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothBleWriteCharacteristic,
            "System.bluetooth.ble",
            "writeCharacteristic",
            vec![generated_argument("sessionId", sessionId), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.ble.writeAndReadCharacteristic` Tools binding.
    fn writeAndReadCharacteristic (& self , sessionId : String , options : SystemBluetoothBleHostWriteAndReadCharacteristicOptions ,) -> JsFuture < BluetoothReadData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothBleWriteAndReadCharacteristic,
            "System.bluetooth.ble",
            "writeAndReadCharacteristic",
            vec![generated_argument("sessionId", sessionId), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.ble.subscribe` Tools binding.
    fn subscribe (& self , sessionId : String , options : SystemBluetoothBleHostSubscribeOptions ,) -> JsFuture < BluetoothTransferData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothBleSubscribeCharacteristic,
            "System.bluetooth.ble",
            "subscribe",
            vec![generated_argument("sessionId", sessionId), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.bluetooth.ble.readNotifications` Tools binding.
    fn readNotifications (& self , sessionId : String , limit : Option < SystemBluetoothBleHostReadNotificationsLimit > ,) -> JsFuture < BluetoothBleNotificationData > {
        invoke_generated(
            self,
            BuiltinToolName::BluetoothBleReadNotifications,
            "System.bluetooth.ble",
            "readNotifications",
            vec![generated_argument("sessionId", sessionId), generated_argument("limit", limit)],
        )
    }

}

impl SystemTerminalHost for AIToolHandler {
    /// Executes the canonical `System.terminal.info` Tools binding.
    fn info (& self) -> JsFuture < TerminalInfoResultData > {
        invoke_generated(
            self,
            BuiltinToolName::GetTerminalInfo,
            "System.terminal",
            "info",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `System.terminal.create` Tools binding.
    fn create (& self , sessionName : String) -> JsFuture < TerminalSessionCreationResultData > {
        invoke_generated(
            self,
            BuiltinToolName::CreateTerminalSession,
            "System.terminal",
            "create",
            vec![generated_argument("sessionName", sessionName)],
        )
    }

    /// Executes the canonical `System.terminal.exec` Tools binding.
    fn exec (& self , sessionId : String , command : String , timeoutMs : Option < SystemTerminalHostExecTimeoutMs > ,) -> JsFuture < TerminalCommandResultData > {
        invoke_generated(
            self,
            BuiltinToolName::ExecuteInTerminalSession,
            "System.terminal",
            "exec",
            vec![generated_argument("sessionId", sessionId), generated_argument("command", command), generated_argument("timeoutMs", timeoutMs)],
        )
    }

    /// Executes the canonical `System.terminal.execStreaming` Tools binding.
    fn execStreaming (& self , sessionId : String , command : String , options : Option < SystemTerminalHostExecStreamingOptions > ,) -> JsFuture < TerminalCommandResultData > {
        invoke_terminal_streaming(
            self,
            BuiltinToolName::ExecuteInTerminalSessionStreaming,
            sessionId,
            command,
            options,
        )
    }

    /// Executes the canonical `System.terminal.hiddenExec` Tools binding.
    fn hiddenExec (& self , command : String , options : Option < SystemTerminalHostHiddenExecOptions > ,) -> JsFuture < HiddenTerminalCommandResultData > {
        invoke_generated(
            self,
            BuiltinToolName::ExecuteHiddenTerminalCommand,
            "System.terminal",
            "hiddenExec",
            vec![generated_argument("command", command), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.terminal.close` Tools binding.
    fn close (& self , sessionId : String) -> JsFuture < TerminalSessionCloseResultData > {
        invoke_generated(
            self,
            BuiltinToolName::CloseTerminalSession,
            "System.terminal",
            "close",
            vec![generated_argument("sessionId", sessionId)],
        )
    }

    /// Executes the canonical `System.terminal.screen` Tools binding.
    fn screen (& self , sessionId : String) -> JsFuture < TerminalSessionScreenResultData > {
        invoke_generated(
            self,
            BuiltinToolName::GetTerminalSessionScreen,
            "System.terminal",
            "screen",
            vec![generated_argument("sessionId", sessionId)],
        )
    }

    /// Executes the canonical `System.terminal.input` Tools binding.
    fn input (& self , sessionId : String , options : Option < SystemTerminalHostInputOptions > ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::InputInTerminalSession,
            "System.terminal",
            "input",
            vec![generated_argument("sessionId", sessionId), generated_argument("options", options)],
        )
    }

}

impl SystemMusicHost for AIToolHandler {
    /// Executes the canonical `System.music.play` Tools binding.
    fn play (& self , options : SystemMusicHostPlayOptions) -> JsFuture < MusicPlaybackResultData > {
        invoke_generated(
            self,
            BuiltinToolName::MusicPlay,
            "System.music",
            "play",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `System.music.pause` Tools binding.
    fn pause (& self) -> JsFuture < MusicPlaybackResultData > {
        invoke_generated(
            self,
            BuiltinToolName::MusicPause,
            "System.music",
            "pause",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `System.music.resume` Tools binding.
    fn resume (& self) -> JsFuture < MusicPlaybackResultData > {
        invoke_generated(
            self,
            BuiltinToolName::MusicResume,
            "System.music",
            "resume",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `System.music.stop` Tools binding.
    fn stop (& self) -> JsFuture < MusicPlaybackResultData > {
        invoke_generated(
            self,
            BuiltinToolName::MusicStop,
            "System.music",
            "stop",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `System.music.seek` Tools binding.
    fn seek (& self , positionMs : SystemMusicHostSeekPositionMs) -> JsFuture < MusicPlaybackResultData > {
        invoke_generated(
            self,
            BuiltinToolName::MusicSeek,
            "System.music",
            "seek",
            vec![generated_argument("positionMs", positionMs)],
        )
    }

    /// Executes the canonical `System.music.setVolume` Tools binding.
    fn setVolume (& self , volume : SystemMusicHostSetVolumeVolume ,) -> JsFuture < MusicPlaybackResultData > {
        invoke_generated(
            self,
            BuiltinToolName::MusicSetVolume,
            "System.music",
            "setVolume",
            vec![generated_argument("volume", volume)],
        )
    }

    /// Executes the canonical `System.music.status` Tools binding.
    fn status (& self) -> JsFuture < MusicPlaybackResultData > {
        invoke_generated(
            self,
            BuiltinToolName::MusicStatus,
            "System.music",
            "status",
            generated_no_arguments(),
        )
    }

}

impl SoftwareSettingsHost for AIToolHandler {
    /// Executes the canonical `SoftwareSettings.readEnvironmentVariable` Tools binding.
    fn readEnvironmentVariable (& self , key : String) -> JsFuture < EnvironmentVariableReadResultData > {
        invoke_generated(
            self,
            BuiltinToolName::ReadEnvironmentVariable,
            "SoftwareSettings",
            "readEnvironmentVariable",
            vec![generated_argument("key", key)],
        )
    }

    /// Executes the canonical `SoftwareSettings.writeEnvironmentVariable` Tools binding.
    fn writeEnvironmentVariable (& self , key : String , value : Option < String > ,) -> JsFuture < EnvironmentVariableWriteResultData > {
        invoke_generated(
            self,
            BuiltinToolName::WriteEnvironmentVariable,
            "SoftwareSettings",
            "writeEnvironmentVariable",
            vec![generated_argument("key", key), generated_argument("value", value)],
        )
    }

    /// Executes the canonical `SoftwareSettings.exec` Tools binding.
    fn exec (& self , args : Vec < String >) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::ExecuteCliCommand,
            "SoftwareSettings",
            "exec",
            vec![generated_argument("args", args)],
        )
    }

}

impl ChatHost for AIToolHandler {
    /// Executes the canonical `Chat.startService` Tools binding.
    fn startService (& self , options : Option < ChatStartServiceOptions > ,) -> JsFuture < ChatServiceStartResultData > {
        invoke_generated(
            self,
            BuiltinToolName::StartChatService,
            "Chat",
            "startService",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Chat.stopService` Tools binding.
    fn stopService (& self) -> JsFuture < ChatServiceStartResultData > {
        invoke_generated(
            self,
            BuiltinToolName::StopChatService,
            "Chat",
            "stopService",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `Chat.createNew` Tools binding.
    fn createNew (& self , group : Option < String > , setAsCurrentChat : Option < bool > , characterCardId : Option < String > ,) -> JsFuture < ChatCreationResultData > {
        invoke_generated(
            self,
            BuiltinToolName::CreateNewChat,
            "Chat",
            "createNew",
            vec![generated_argument("group", group), generated_argument("setAsCurrentChat", setAsCurrentChat), generated_argument("characterCardId", characterCardId)],
        )
    }

    /// Executes the canonical `Chat.listAll` Tools binding.
    fn listAll (& self) -> JsFuture < ChatListResultData > {
        invoke_generated(
            self,
            BuiltinToolName::ListChats,
            "Chat",
            "listAll",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `Chat.listChats` Tools binding.
    fn listChats (& self , params : Option < ChatHostListChatsParams >) -> JsFuture < ChatListResultData > {
        invoke_generated(
            self,
            BuiltinToolName::ListChats,
            "Chat",
            "listChats",
            vec![generated_argument("params", params)],
        )
    }

    /// Executes the canonical `Chat.findChat` Tools binding.
    fn findChat (& self , params : ChatHostFindChatParams) -> JsFuture < ChatFindResultData > {
        invoke_generated(
            self,
            BuiltinToolName::FindChat,
            "Chat",
            "findChat",
            vec![generated_argument("params", params)],
        )
    }

    /// Executes the canonical `Chat.agentStatus` Tools binding.
    fn agentStatus (& self , chatId : String) -> JsFuture < AgentStatusResultData > {
        invoke_generated(
            self,
            BuiltinToolName::AgentStatus,
            "Chat",
            "agentStatus",
            vec![generated_argument("chatId", chatId)],
        )
    }

    /// Executes the canonical `Chat.switchTo` Tools binding.
    fn switchTo (& self , chatId : String) -> JsFuture < ChatSwitchResultData > {
        invoke_generated(
            self,
            BuiltinToolName::SwitchChat,
            "Chat",
            "switchTo",
            vec![generated_argument("chatId", chatId)],
        )
    }

    /// Executes the canonical `Chat.updateTitle` Tools binding.
    fn updateTitle (& self , chatId : String , title : String) -> JsFuture < ChatTitleUpdateResultData > {
        invoke_generated(
            self,
            BuiltinToolName::UpdateChatTitle,
            "Chat",
            "updateTitle",
            vec![generated_argument("chatId", chatId), generated_argument("title", title)],
        )
    }

    /// Executes the canonical `Chat.deleteChat` Tools binding.
    fn deleteChat (& self , chatId : String) -> JsFuture < ChatDeleteResultData > {
        invoke_generated(
            self,
            BuiltinToolName::DeleteChat,
            "Chat",
            "deleteChat",
            vec![generated_argument("chatId", chatId)],
        )
    }

    /// Executes the canonical `Chat.sendMessage` Tools binding.
    fn sendMessage (& self , message : String , chatId : Option < String > , roleCardId : Option < String > , senderName : Option < String > , options : Option < ChatSendMessageOptions > ,) -> JsFuture < MessageSendResultData > {
        invoke_generated(
            self,
            BuiltinToolName::SendMessageToAi,
            "Chat",
            "sendMessage",
            vec![generated_argument("message", message), generated_argument("chatId", chatId), generated_argument("roleCardId", roleCardId), generated_argument("senderName", senderName), generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Chat.sendMessageStreaming` Tools binding.
    fn sendMessageStreaming (& self , message : String , chatId : Option < String > , roleCardId : Option < String > , senderName : Option < String > , options : Option < ChatSendMessageStreamingOptions > ,) -> JsFuture < MessageSendResultData > {
        invoke_chat_streaming(
            self,
            BuiltinToolName::SendMessageToAiStreaming,
            message,
            chatId,
            roleCardId,
            senderName,
            options,
        )
    }

    /// Executes the canonical `Chat.listCharacterCards` Tools binding.
    fn listCharacterCards (& self) -> JsFuture < CharacterCardListResultData > {
        invoke_generated(
            self,
            BuiltinToolName::ListCharacterCards,
            "Chat",
            "listCharacterCards",
            generated_no_arguments(),
        )
    }

    /// Executes the canonical `Chat.getMessages` Tools binding.
    fn getMessages (& self , chatId : String , options : Option < ChatHostGetMessagesOptions > ,) -> JsFuture < ChatMessagesResultData > {
        invoke_generated(
            self,
            BuiltinToolName::GetChatMessages,
            "Chat",
            "getMessages",
            vec![generated_argument("chatId", chatId), generated_argument("options", options)],
        )
    }

}

impl MemoryHost for AIToolHandler {
    /// Executes the canonical `Memory.query` Tools binding.
    fn query_overload_1 (& self , query : String , folderPath : Option < String > , limit : Option < f64 > , startTime : Option < String > , endTime : Option < String > , snapshotId : Option < String > , threshold : Option < f64 > , targetOwnerKey : Option < String > ,) -> JsFuture < MemoryQueryResultData > {
        invoke_generated(
            self,
            BuiltinToolName::QueryMemory,
            "Memory",
            "query",
            vec![generated_argument("query", query), generated_argument("folderPath", folderPath), generated_argument("limit", limit), generated_argument("startTime", startTime), generated_argument("endTime", endTime), generated_argument("snapshotId", snapshotId), generated_argument("threshold", threshold), generated_argument("targetOwnerKey", targetOwnerKey)],
        )
    }

    /// Executes the canonical `Memory.query` Tools binding.
    fn query_overload_2 (& self , options : MemoryQueryOptions) -> JsFuture < MemoryQueryResultData > {
        invoke_generated(
            self,
            BuiltinToolName::QueryMemory,
            "Memory",
            "query",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Memory.getByTitle` Tools binding.
    fn getByTitle_overload_1 (& self , title : String , targetOwnerKey : String , chunkIndex : Option < f64 > , chunkRange : Option < String > , query : Option < String > , limit : Option < f64 > ,) -> JsFuture < MemoryQueryResultData > {
        invoke_generated(
            self,
            BuiltinToolName::GetMemoryByTitle,
            "Memory",
            "getByTitle",
            vec![generated_argument("title", title), generated_argument("targetOwnerKey", targetOwnerKey), generated_argument("chunkIndex", chunkIndex), generated_argument("chunkRange", chunkRange), generated_argument("query", query), generated_argument("limit", limit)],
        )
    }

    /// Executes the canonical `Memory.getByTitle` Tools binding.
    fn getByTitle_overload_2 (& self , options : MemoryGetByTitleOptions ,) -> JsFuture < MemoryQueryResultData > {
        invoke_generated(
            self,
            BuiltinToolName::GetMemoryByTitle,
            "Memory",
            "getByTitle",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Memory.create` Tools binding.
    fn create_overload_1 (& self , title : String , content : String , targetOwnerKey : String , contentType : Option < String > , source : Option < String > , folderPath : Option < String > , tags : Option < String > ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::CreateMemory,
            "Memory",
            "create",
            vec![generated_argument("title", title), generated_argument("content", content), generated_argument("targetOwnerKey", targetOwnerKey), generated_argument("contentType", contentType), generated_argument("source", source), generated_argument("folderPath", folderPath), generated_argument("tags", tags)],
        )
    }

    /// Executes the canonical `Memory.create` Tools binding.
    fn create_overload_2 (& self , options : MemoryCreateOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::CreateMemory,
            "Memory",
            "create",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Memory.update` Tools binding.
    fn update_overload_1 (& self , oldTitle : String , targetOwnerKey : String , updates : Option < MemoryUpdateOptions > ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::UpdateMemory,
            "Memory",
            "update",
            vec![generated_argument("oldTitle", oldTitle), generated_argument("targetOwnerKey", targetOwnerKey), generated_argument("updates", updates)],
        )
    }

    /// Executes the canonical `Memory.update` Tools binding.
    fn update_overload_2 (& self , options : MemoryHostUpdateOptionsIntersection) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::UpdateMemory,
            "Memory",
            "update",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Memory.updateUserPreferences` Tools binding.
    fn updateUserPreferences_overload_1 (& self , content : String , targetOwnerKey : String ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::UpdateUserPreferences,
            "Memory",
            "updateUserPreferences",
            vec![generated_argument("content", content), generated_argument("targetOwnerKey", targetOwnerKey)],
        )
    }

    /// Executes the canonical `Memory.updateUserPreferences` Tools binding.
    fn updateUserPreferences_overload_2 (& self , options : MemoryUserPreferencesOptions ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::UpdateUserPreferences,
            "Memory",
            "updateUserPreferences",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Memory.deleteMemory` Tools binding.
    fn deleteMemory_overload_1 (& self , title : String , targetOwnerKey : String) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::DeleteMemory,
            "Memory",
            "deleteMemory",
            vec![generated_argument("title", title), generated_argument("targetOwnerKey", targetOwnerKey)],
        )
    }

    /// Executes the canonical `Memory.deleteMemory` Tools binding.
    fn deleteMemory_overload_2 (& self , options : MemoryDeleteOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::DeleteMemory,
            "Memory",
            "deleteMemory",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Memory.move` Tools binding.
    fn move_overload_1 (& self , targetFolderPath : String , targetOwnerKey : String , titles : Option < MemoryHostMoveTitles > , sourceFolderPath : Option < String > ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::MoveMemory,
            "Memory",
            "move",
            vec![generated_argument("targetFolderPath", targetFolderPath), generated_argument("targetOwnerKey", targetOwnerKey), generated_argument("titles", titles), generated_argument("sourceFolderPath", sourceFolderPath)],
        )
    }

    /// Executes the canonical `Memory.move` Tools binding.
    fn move_overload_2 (& self , options : MemoryMoveOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::MoveMemory,
            "Memory",
            "move",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Memory.link` Tools binding.
    fn link_overload_1 (& self , sourceTitle : String , targetTitle : String , targetOwnerKey : String , linkType : Option < String > , weight : Option < f64 > , description : Option < String > ,) -> JsFuture < MemoryLinkResultData > {
        invoke_generated(
            self,
            BuiltinToolName::LinkMemories,
            "Memory",
            "link",
            vec![generated_argument("sourceTitle", sourceTitle), generated_argument("targetTitle", targetTitle), generated_argument("targetOwnerKey", targetOwnerKey), generated_argument("linkType", linkType), generated_argument("weight", weight), generated_argument("description", description)],
        )
    }

    /// Executes the canonical `Memory.link` Tools binding.
    fn link_overload_2 (& self , options : MemoryLinkOptions) -> JsFuture < MemoryLinkResultData > {
        invoke_generated(
            self,
            BuiltinToolName::LinkMemories,
            "Memory",
            "link",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Memory.queryLinks` Tools binding.
    fn queryLinks_overload_1 (& self , targetOwnerKey : String , linkId : Option < f64 > , sourceTitle : Option < String > , targetTitle : Option < String > , linkType : Option < String > , limit : Option < f64 > ,) -> JsFuture < MemoryLinkQueryResultData > {
        invoke_generated(
            self,
            BuiltinToolName::QueryMemoryLinks,
            "Memory",
            "queryLinks",
            vec![generated_argument("targetOwnerKey", targetOwnerKey), generated_argument("linkId", linkId), generated_argument("sourceTitle", sourceTitle), generated_argument("targetTitle", targetTitle), generated_argument("linkType", linkType), generated_argument("limit", limit)],
        )
    }

    /// Executes the canonical `Memory.queryLinks` Tools binding.
    fn queryLinks_overload_2 (& self , options : MemoryQueryLinksOptions ,) -> JsFuture < MemoryLinkQueryResultData > {
        invoke_generated(
            self,
            BuiltinToolName::QueryMemoryLinks,
            "Memory",
            "queryLinks",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Memory.updateLink` Tools binding.
    fn updateLink_overload_1 (& self , targetOwnerKey : String , linkId : Option < f64 > , sourceTitle : Option < String > , targetTitle : Option < String > , linkType : Option < String > , newLinkType : Option < String > , weight : Option < f64 > , description : Option < String > ,) -> JsFuture < MemoryLinkQueryResultData > {
        invoke_generated(
            self,
            BuiltinToolName::UpdateMemoryLink,
            "Memory",
            "updateLink",
            vec![generated_argument("targetOwnerKey", targetOwnerKey), generated_argument("linkId", linkId), generated_argument("sourceTitle", sourceTitle), generated_argument("targetTitle", targetTitle), generated_argument("linkType", linkType), generated_argument("newLinkType", newLinkType), generated_argument("weight", weight), generated_argument("description", description)],
        )
    }

    /// Executes the canonical `Memory.updateLink` Tools binding.
    fn updateLink_overload_2 (& self , options : MemoryUpdateLinkOptions ,) -> JsFuture < MemoryLinkQueryResultData > {
        invoke_generated(
            self,
            BuiltinToolName::UpdateMemoryLink,
            "Memory",
            "updateLink",
            vec![generated_argument("options", options)],
        )
    }

    /// Executes the canonical `Memory.deleteLink` Tools binding.
    fn deleteLink_overload_1 (& self , targetOwnerKey : String , linkId : Option < f64 > , sourceTitle : Option < String > , targetTitle : Option < String > , linkType : Option < String > ,) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::DeleteMemoryLink,
            "Memory",
            "deleteLink",
            vec![generated_argument("targetOwnerKey", targetOwnerKey), generated_argument("linkId", linkId), generated_argument("sourceTitle", sourceTitle), generated_argument("targetTitle", targetTitle), generated_argument("linkType", linkType)],
        )
    }

    /// Executes the canonical `Memory.deleteLink` Tools binding.
    fn deleteLink_overload_2 (& self , options : MemoryDeleteLinkOptions) -> JsFuture < String > {
        invoke_generated(
            self,
            BuiltinToolName::DeleteMemoryLink,
            "Memory",
            "deleteLink",
            vec![generated_argument("options", options)],
        )
    }

}

