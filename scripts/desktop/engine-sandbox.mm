#import <Foundation/Foundation.h>
#include <cstdio>
#include <cstdlib>
#include <unistd.h>

// Resolve the parent's transferable bookmark inside the engine process, before model discovery.
__attribute__((constructor)) static void desktop_sandbox_access() {
    @autoreleasepool {
        const char *encoded = std::getenv("MUSIC_MODEL_BOOKMARK");
        if (encoded && *encoded) {
            NSData *data = [[NSData alloc] initWithBase64EncodedString:[NSString stringWithUTF8String:encoded] options:0];
            NSError *error = nil;
            NSURL *url = [NSURL URLByResolvingBookmarkData:data options:NSURLBookmarkResolutionWithoutUI relativeToURL:nil bookmarkDataIsStale:nil error:&error];
            if (!url) {
                std::fprintf(stderr, "[Desktop Sandbox] Model folder access: %s\n", error.localizedDescription.UTF8String);
                _exit(73);
            }
            // Non-security-scoped creation options carry an implicit sandbox extension to this process.
            [url startAccessingSecurityScopedResource];
            // The engine owns this grant until it exits; keep the URL alive beyond the autorelease pool.
            CFRetain((__bridge CFTypeRef)url);
        }
        if (const char *denied = std::getenv("MUSIC_SANDBOX_DENIED_PATH")) {
            if (FILE *file = std::fopen(denied, "rb")) {
                std::fclose(file);
                std::fprintf(stderr, "[Desktop Sandbox] ERROR: engine could read unselected file\n");
                _exit(74);
            }
            std::fprintf(stderr, "[Desktop Sandbox] Unselected file access denied\n");
        }
    }
}
