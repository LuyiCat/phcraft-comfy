// The system document picker, driven from safe Rust through a small C ABI (src/picker.rs).
// The delegates live here, not in Rust, because implementing an ObjC protocol needs objc2's
// declare_class! (unsafe); everything UIKit stays in this file, force-linked like
// PCSceneDelegate.m. Open picks are read inside the delegate (security scope) and delivered
// as (name, bytes) callbacks; Save As exports an empty staging file and reports the picked
// destination back to Rust. Mode is tracked so an export's didPick is not mistaken for an open.
#import <UIKit/UIKit.h>

typedef void (*pc_open_cb)(const char *name, const uint8_t *bytes, size_t len, int last);
typedef void (*pc_cancel_cb)(void);

@class PCDocumentPickerDelegate;
static UIViewController *PCRootViewController(void);

// Declared early: pc_pick_open/pc_pick_export below reference the shared delegate.
@interface PCDocumentPickerDelegate : NSObject <UIDocumentPickerDelegate>
+ (instancetype)shared;
@end

static pc_open_cb PCOpenCallback;
static pc_cancel_cb PCCancelCallback;
static BOOL PCExportMode;

// Save As destinations live outside the app sandbox. Keep bookmarks for them so File ▸ Save
// can regain the picker-granted access after a relaunch while iOS's implicit scope is valid.
static NSMutableDictionary<NSString *, NSData *> *PCBookmarkData;
static NSMutableDictionary<NSString *, NSURL *> *PCScopedURLs;

static NSURL *PCBookmarkStoreURL(void) {
    NSURL *base = [[NSFileManager defaultManager] URLsForDirectory:NSApplicationSupportDirectory
                                                         inDomains:NSUserDomainMask].firstObject;
    if (base == nil) return nil;
    NSURL *dir = [base URLByAppendingPathComponent:@"PhotoComfy" isDirectory:YES];
    [[NSFileManager defaultManager] createDirectoryAtURL:dir
                             withIntermediateDirectories:YES
                                              attributes:nil
                                                   error:nil];
    return [dir URLByAppendingPathComponent:@"external-bookmarks.plist"];
}

static void PCSaveBookmarks(void) {
    NSURL *url = PCBookmarkStoreURL();
    if (url == nil || PCBookmarkData == nil) return;
    [[NSDictionary dictionaryWithDictionary:PCBookmarkData] writeToURL:url atomically:YES];
}

static void PCLoadBookmarks(void) {
    PCBookmarkData = [NSMutableDictionary dictionary];
    PCScopedURLs = [NSMutableDictionary dictionary];
    NSURL *store = PCBookmarkStoreURL();
    NSDictionary *raw = store == nil ? nil : [NSDictionary dictionaryWithContentsOfURL:store];
    BOOL changed = NO;
    for (NSString *path in raw) {
        NSData *data = raw[path];
        if (![path isKindOfClass:NSString.class] || ![data isKindOfClass:NSData.class]) {
            changed = YES;
            continue;
        }
        BOOL stale = NO;
        NSError *error = nil;
        NSURL *url = [NSURL URLByResolvingBookmarkData:data
                                               options:NSURLBookmarkResolutionWithoutUI |
                                                        NSURLBookmarkResolutionWithoutImplicitStartAccessing
                                         relativeToURL:nil
                                  bookmarkDataIsStale:&stale
                                                 error:&error];
        if (url == nil || ![url startAccessingSecurityScopedResource]) {
            changed = YES;
            continue;
        }
        PCScopedURLs[path] = url;
        if (stale) {
            NSData *fresh = [url bookmarkDataWithOptions:NSURLBookmarkCreationMinimalBookmark
                          includingResourceValuesForKeys:nil
                                           relativeToURL:nil
                                                   error:nil];
            if (fresh != nil) PCBookmarkData[path] = fresh;
            changed = YES;
        } else {
            PCBookmarkData[path] = data;
        }
    }
    if (changed) PCSaveBookmarks();
}

static void PCRememberDestination(NSURL *url) {
    NSString *path = url.path;
    if (path == nil) return;
    // The access intentionally stays active for the process; the retained URL keeps the
    // picker grant alive, and the bookmark restores it on the next launch.
    if (![url startAccessingSecurityScopedResource]) return;
    PCScopedURLs[path] = url;
    NSData *data = [url bookmarkDataWithOptions:NSURLBookmarkCreationMinimalBookmark
                 includingResourceValuesForKeys:nil
                                  relativeToURL:nil
                                          error:nil];
    if (data != nil) {
        PCBookmarkData[path] = data;
        PCSaveBookmarks();
    }
}

__attribute__((visibility("default"))) void pc_set_callbacks(pc_open_cb open, pc_cancel_cb cancel) {
    PCOpenCallback = open;
    PCCancelCallback = cancel;
    PCLoadBookmarks();
}

__attribute__((visibility("default"))) void pc_pick_open(int allow_multiple) {
    dispatch_async(dispatch_get_main_queue(), ^{
        UIViewController *root = PCRootViewController();
        if (root == nil || PCOpenCallback == NULL) {
            if (PCCancelCallback != NULL) PCCancelCallback();
            return;
        }
        PCExportMode = NO;
        UIDocumentPickerViewController *picker = [[UIDocumentPickerViewController alloc]
            initWithDocumentTypes:@[ @"public.data" ]   // every concrete file type conforms; PSD/PNG/JPEG included
                            inMode:UIDocumentPickerModeOpen];
        picker.delegate = [PCDocumentPickerDelegate shared];
        picker.allowsMultipleSelection = (allow_multiple != 0);
        [root presentViewController:picker animated:YES completion:nil];
    });
}

// Present the export sheet for a file the shell has already written; the system copies it to
// the chosen destination (asCopy leaves our file alone). Save As stages an empty placeholder
// through the same path and reads the picked destination from the delegate callback.
__attribute__((visibility("default"))) void pc_pick_export(const char *path) {
    // Copy the path BEFORE dispatching. The Rust caller frees its CString when the call
    // returns, and the block below runs on a later main-queue turn: capturing the raw pointer
    // dereferenced freed memory and handed UIKit a garbage path, which it asserts on
    // (NSInternalInconsistencyException) rather than reporting — every save aborted the app.
    NSString *filePath = path == NULL ? nil : [NSString stringWithUTF8String:path];
    dispatch_async(dispatch_get_main_queue(), ^{
        UIViewController *root = PCRootViewController();
        // UIKit's export initializer REQUIRES an existing file and throws when it isn't one,
        // so a vanished/never-written file must cancel here, not reach the initializer.
        if (root == nil || filePath.length == 0
            || ![[NSFileManager defaultManager] fileExistsAtPath:filePath]) {
            if (PCCancelCallback != NULL) PCCancelCallback();
            return;
        }
        PCExportMode = YES;
        NSURL *url = [NSURL fileURLWithPath:filePath];
        UIDocumentPickerViewController *picker =
            [[UIDocumentPickerViewController alloc] initForExportingURLs:@[ url ] asCopy:YES];
        picker.delegate = [PCDocumentPickerDelegate shared];
        [root presentViewController:picker animated:YES completion:nil];
    });
}

// winit's classic path keeps no root view controller pointer Rust could reach: find the first
// window of the first connected window scene (the one PCSceneDelegate attached).
static UIViewController *PCRootViewController(void) {
    for (UIScene *scene in UIApplication.sharedApplication.connectedScenes) {
        if (![scene isKindOfClass:UIWindowScene.class]) continue;
        UIWindowScene *windowScene = (UIWindowScene *)scene;
        if (windowScene.keyWindow.rootViewController != nil) {
            return windowScene.keyWindow.rootViewController;
        }
        for (UIWindow *window in windowScene.windows) {
            if (window.rootViewController != nil) return window.rootViewController;
        }
    }
    return nil;
}

@implementation PCDocumentPickerDelegate

static PCDocumentPickerDelegate *PCSharedDelegate;

+ (instancetype)shared {
    if (PCSharedDelegate == nil) PCSharedDelegate = [[self alloc] init];
    return PCSharedDelegate;
}

- (void)documentPicker:(UIDocumentPickerViewController *)controller
    didPickDocumentsAtURLs:(NSArray<NSURL *> *)urls {
    [controller dismissViewControllerAnimated:YES completion:nil];
    if (PCExportMode) {
        // Save As needs the real destination path: the system has copied the staged
        // placeholder there, and the app overwrites that copy with the document bytes.
        NSURL *url = urls.firstObject;
        if (url != nil) {
            PCRememberDestination(url);
            NSString *path = url.path ?: @"";
            if (PCOpenCallback != NULL) PCOpenCallback(path.UTF8String, NULL, 0, 1);
        } else if (PCOpenCallback != NULL) {
            PCOpenCallback("", NULL, 0, 1);
        }
        return;
    }
    NSUInteger last = urls.count - 1;
    for (NSUInteger index = 0; index < urls.count; index++) {
        NSURL *url = urls[index];
        NSString *name = url.lastPathComponent ?: @"file";
        NSData *data = nil;
        BOOL scoped = [url startAccessingSecurityScopedResource];
        data = [NSData dataWithContentsOfURL:url options:0 error:nil];
        if (scoped) [url stopAccessingSecurityScopedResource];
        if (PCOpenCallback == NULL) return;
        if (data != nil) {
            PCOpenCallback(name.UTF8String ?: "file", data.bytes, data.length, (int)(index == last));
        } else if (index == last) {
            PCOpenCallback("", NULL, 0, 1);  // unreadable selection: resolve as empty, not a hang
        }
    }
}

- (void)documentPickerWasCancelled:(UIDocumentPickerViewController *)controller {
    [controller dismissViewControllerAnimated:YES completion:nil];
    if (PCCancelCallback != NULL) PCCancelCallback();
}

@end
