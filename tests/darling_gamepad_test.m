// GCController through the Quadra shim in real Darling. The shim is inserted
// with DYLD_INSERT_LIBRARIES; tests/run_darling_gamepad.sh creates and moves a
// virtual gamepad while this runs and reads the progress lines printed here.
extern int printf(const char *, ...);
extern int fflush(void *);
extern void exit(int);
typedef struct objc_class *Class;
typedef struct objc_object { Class isa; } *id;
typedef struct objc_selector *SEL;
typedef signed char BOOL;
typedef long NSInteger;
typedef unsigned long NSUInteger;
#define YES 1
#define NO 0
#define nil ((id)0)
extern Class NSClassFromString(id);
extern SEL NSSelectorFromString(id);
@interface NSObject
+ (id)alloc;
+ (id)new;
- (id)init;
- (id)retain;
- (void)release;
- (id)copy;
- (id)performSelector:(SEL)s withObject:(id)o;
@end
@interface NSAutoreleasePool : NSObject @end
@interface NSString : NSObject
- (BOOL)isEqualToString:(id)s;
@end
@interface NSArray : NSObject
- (NSUInteger)count;
- (id)objectAtIndex:(NSUInteger)i;
@end
@interface NSDictionary : NSObject
- (id)objectForKey:(id)k;
@end
@interface NSDate : NSObject
+ (id)dateWithTimeIntervalSinceNow:(double)s;
- (double)timeIntervalSinceNow;
@end
@interface NSRunLoop : NSObject
+ (id)currentRunLoop;
- (void)runUntilDate:(id)d;
@end
@interface NSNotification : NSObject
- (id)object;
@end
@interface NSNotificationCenter : NSObject
+ (id)defaultCenter;
- (id)addObserverForName:(id)n object:(id)o queue:(id)q usingBlock:(void (^)(NSNotification *))b;
@end

@interface NSObject (GCProbe)
+ (NSArray *)controllers;
- (id)extendedGamepad;
- (id)buttonA;
- (id)leftThumbstick;
- (id)xAxis;
- (id)yAxis;
- (id)leftTrigger;
- (id)dpad;
- (id)left;
- (BOOL)isPressed;
- (float)value;
- (id)vendorName;
- (id)productCategory;
- (NSInteger)playerIndex;
- (id)physicalInputProfile;
- (id)buttons;
@end

extern id GCInputButtonA, GCInputLeftThumbstick;

static BOOL wait_until(BOOL (^condition)(void), double seconds) {
    NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:seconds];
    while (!condition()) {
        if ([deadline timeIntervalSinceNow] <= 0) return NO;
        [[NSRunLoop currentRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.05]];
    }
    return YES;
}

static void check(BOOL ok, const char *what) {
    if (!ok) { printf("FAIL %s\n", what); fflush(0); exit(1); }
}

int main(void) {
    NSAutoreleasePool *pool = [[NSAutoreleasePool alloc] init];
    {
        Class gc = NSClassFromString(@"GCController");
        check(gc != nil, "GCController class loads");
        check([[gc controllers] count] == 0, "no controllers before the pad exists");

        __block id connected = nil, disconnected = nil;
        __block int value_changes = 0;
        [[NSNotificationCenter defaultCenter] addObserverForName:@"GCControllerDidConnectNotification" object:nil
            queue:nil usingBlock:^(NSNotification *n) { connected = [[n object] retain]; }];
        [[NSNotificationCenter defaultCenter] addObserverForName:@"GCControllerDidDisconnectNotification" object:nil
            queue:nil usingBlock:^(NSNotification *n) { disconnected = [[n object] retain]; }];
        printf("WAITING\n"); fflush(0);

        check(wait_until(^BOOL{ return connected != nil; }, 30), "connect notification");
        check([[gc controllers] count] == 1 && [[gc controllers] objectAtIndex:0] == connected, "controllers lists the pad");
        check([[connected vendorName] isEqualToString:@"Quadra Test Gamepad"], "vendorName");
        check([[connected productCategory] isEqualToString:@"Xbox One"], "productCategory");
        check([connected playerIndex] == -1, "player index starts unset");
        id pad = [connected extendedGamepad];
        check(pad != nil && [connected physicalInputProfile] == pad, "extendedGamepad and physicalInputProfile");
        check([[pad buttonA] isPressed] == NO, "A released at connect");
        check([[pad buttons] objectForKey:@"Button A"] == [pad buttonA], "named button lookup");
        // The constants the client looks buttons up by; Darling's framework has none.
        check(GCInputButtonA != nil && [[pad buttons] objectForKey:GCInputButtonA] == [pad buttonA],
              "GCInputButtonA key");
        printf("CONNECTED\n"); fflush(0);

        // The handler is set after connect, so it sees only the change below.
        id button = [pad buttonA];
        [button performSelector:NSSelectorFromString(@"setValueChangedHandler:")
                     withObject:[^(id b, float v, BOOL p) { value_changes++; } copy]];
        check(wait_until(^BOOL{ return [[pad buttonA] isPressed] && [[[pad leftThumbstick] xAxis] value] > 0.99f; }, 20),
              "A pressed and stick right");
        check([[[pad leftThumbstick] yAxis] value] > 0.99f, "stick up is positive");
        check(wait_until(^BOOL{ return [[pad leftTrigger] value] > 0.99f && [[[pad dpad] left] isPressed]; }, 20),
              "trigger and d-pad left");
        check(value_changes >= 1, "valueChangedHandler ran");
        printf("INPUT\n"); fflush(0);

        check(wait_until(^BOOL{ return disconnected != nil; }, 30), "disconnect notification");
        check(disconnected == connected && [[gc controllers] count] == 0, "pad removed on unplug");
        printf("ok\n"); fflush(0);
    }
    return 0;
}
