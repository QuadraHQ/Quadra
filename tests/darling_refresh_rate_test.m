/* QUADRA_REFRESH_RATE through the shim in real Darling: the screen reports the
 * overridden rate and matching intervals. Run by tests/run_darling_refresh_rate.sh
 * with the shim inserted and a disposable X server. */
extern int printf(const char *, ...);
extern int fflush(void *);
typedef struct objc_class *Class;
typedef struct objc_object { Class isa; } *id;
typedef signed char BOOL;
@interface NSObject
+ (id)alloc;
- (id)init;
@end
@interface NSAutoreleasePool : NSObject @end
@interface NSScreen : NSObject
+ (id)mainScreen;
- (long)maximumFramesPerSecond;
- (double)minimumRefreshInterval;
- (double)maximumRefreshInterval;
- (double)displayUpdateGranularity;
@end

int main(void) {
    NSAutoreleasePool *pool = [[NSAutoreleasePool alloc] init];
    printf("start\n"); fflush(0);
    NSScreen *screen = [NSScreen mainScreen];
    printf("have screen %p\n", (void *)screen); fflush(0);
    long fps = [screen maximumFramesPerSecond];
    double interval = [screen minimumRefreshInterval];
    printf("fps=%ld min=%.6f max=%.6f granularity=%.6f\n", fps, interval,
           [screen maximumRefreshInterval], [screen displayUpdateGranularity]);
    int ok = fps == 144 && interval > 0.00694 && interval < 0.00695 &&
             [screen maximumRefreshInterval] == interval && [screen displayUpdateGranularity] == interval;
    printf(ok ? "ok\n" : "FAIL\n"); fflush(0);
    (void)pool;
    return ok ? 0 : 1;
}
