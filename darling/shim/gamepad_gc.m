/* GCController for Linux gamepads.
 *
 * Darling's GameController framework is stubs (every class forwards unknown
 * messages to an NSLog), so +[GCController controllers] cannot return a pad.
 * This replaces the class methods RobloxPlayer uses and hands back controller
 * objects that follow GCController's extended-gamepad API, fed by
 * gamepad_evdev.c. Compiled without ARC, like the rest of the shim.
 *
 * Handlers and notifications run on the main queue, as GameController's
 * default handlerQueue does. Not implemented: motion, light, haptics, battery
 * and the micro gamepad profile (those properties return nil), and snapshots.
 * QUADRA_GAMEPAD=0 turns the bridge off.
 */
#include "gamepad_evdev.h"

/* The Darling runtime ships no headers: declare what is used, as web_bridge.m
 * does. These are Foundation's and libdispatch's real signatures. */
typedef struct objc_class *Class;
typedef struct objc_object { Class isa; } *id;
typedef struct objc_selector *SEL;
typedef void (*IMP)(void);
typedef signed char BOOL;
typedef unsigned long NSUInteger;
typedef long NSInteger;
#define YES 1
#define NO 0
#define nil ((id)0)

extern Class objc_getClass(const char *);
extern Class object_getClass(id);
extern SEL sel_registerName(const char *);
extern IMP class_replaceMethod(Class, SEL, IMP, const char *);
extern char *getenv(const char *);
extern struct dispatch_queue_s _dispatch_main_q;
extern void dispatch_async(void *, void (^)(void));
extern void dispatch_retain(void *);
extern void dispatch_release(void *);
extern int pthread_create(void **, const void *, void *(*)(void *), void *);
extern int pthread_detach(void *);
extern int pthread_setname_np(const char *);
extern void _dyld_register_func_for_add_image(void (*)(const void *, long));
#define dispatch_get_main_queue() ((void *)&_dispatch_main_q)

@interface NSObject { Class isa; }
+ (id)alloc;
- (id)init;
- (id)retain;
- (oneway void)release;
- (void)dealloc;
- (id)copy;
@end
@interface NSString : NSObject
+ (id)stringWithUTF8String:(const char *)s;
- (id)stringByAppendingString:(id)s;
@end
@interface NSArray : NSObject
+ (id)arrayWithArray:(id)a;
- (id)firstObject;
@end
@interface NSMutableArray : NSArray
- (void)addObject:(id)o;
- (void)removeObject:(id)o;
@end
@interface NSSet : NSObject
+ (id)set;
+ (id)setWithObject:(id)o;
+ (id)setWithArray:(id)a;
- (id)setByAddingObjectsFromSet:(id)s;
@end
@interface NSDictionary : NSObject
+ (id)dictionary;
+ (id)dictionaryWithObjectsAndKeys:(id)first, ...;
- (id)objectForKey:(id)k;
- (NSArray *)allValues;
@end
@interface NSMutableDictionary : NSDictionary
+ (id)dictionary;
+ (id)dictionaryWithDictionary:(id)d;
- (void)setObject:(id)o forKey:(id)k;
- (void)addEntriesFromDictionary:(id)d;
@end
@interface NSNotificationCenter : NSObject
+ (id)defaultCenter;
- (void)postNotificationName:(id)name object:(id)object;
@end


static const float kPressedThreshold = 0.1f;

@class QGCController;

@interface QGCElement : NSObject {
    @public
    NSString *_name;
}
- (BOOL)isAnalog;
- (NSString *)localizedName;
@end

@interface QGCButton : QGCElement {
    @public
    float _value;
    id _valueChangedHandler, _pressedChangedHandler;
    BOOL _analog;
}
- (float)value;
- (BOOL)isPressed;
- (void)applyValue:(float)value;
@end

@interface QGCAxis : QGCElement {
    @public
    float _value;
    id _valueChangedHandler;
}
- (float)value;
- (void)applyValue:(float)value;
@end

@interface QGCDirectionPad : QGCElement {
    @public
    QGCButton *_up, *_down, *_left, *_right;
    QGCAxis *_xAxis, *_yAxis;
    id _valueChangedHandler;
}
- (void)applyX:(float)x y:(float)y;
@end

@interface QGCGamepad : NSObject {
    @public
    QGCController *_controller; /* not retained: the controller owns this */
    QGCButton *_buttonA, *_buttonB, *_buttonX, *_buttonY;
    QGCButton *_leftShoulder, *_rightShoulder, *_leftTrigger, *_rightTrigger;
    QGCButton *_menu, *_options, *_home, *_leftStickButton, *_rightStickButton;
    QGCDirectionPad *_dpad, *_leftThumbstick, *_rightThumbstick;
    id _valueChangedHandler;
}
- (void)applyPad:(const qgp_pad *)pad;
@end

@interface QGCController : NSObject {
    @public
    int _slot;
    NSString *_vendorName, *_category;
    NSInteger _playerIndex;
    QGCGamepad *_gamepad;
    id _pausedHandler;
    void *_handlerQueue;
}
@end

@implementation QGCElement
- (BOOL)isAnalog { return YES; }
- (NSString *)localizedName { return _name; }
- (NSString *)sfSymbolsName { return nil; }
- (NSSet *)aliases { return [NSSet setWithObject:_name]; }
- (void)dealloc { [_name release]; [super dealloc]; }
@end

@implementation QGCButton
- (float)value { return _value; }
- (BOOL)isPressed { return _value > kPressedThreshold; }
- (BOOL)isAnalog { return _analog; }
- (id)valueChangedHandler { return _valueChangedHandler; }
- (void)setValueChangedHandler:(id)h { id old = _valueChangedHandler; _valueChangedHandler = [h copy]; [old release]; }
- (id)pressedChangedHandler { return _pressedChangedHandler; }
- (void)setPressedChangedHandler:(id)h { id old = _pressedChangedHandler; _pressedChangedHandler = [h copy]; [old release]; }
- (void)applyValue:(float)value {
    if (value == _value) return;
    BOOL was = [self isPressed];
    _value = value;
    BOOL now = [self isPressed];
    void (^changed)(id, float, BOOL) = _valueChangedHandler;
    if (changed) changed(self, value, now);
    void (^pressed)(id, float, BOOL) = _pressedChangedHandler;
    if (pressed && was != now) pressed(self, value, now);
}
- (void)dealloc { [_valueChangedHandler release]; [_pressedChangedHandler release]; [super dealloc]; }
@end

@implementation QGCAxis
- (float)value { return _value; }
- (id)valueChangedHandler { return _valueChangedHandler; }
- (void)setValueChangedHandler:(id)h { id old = _valueChangedHandler; _valueChangedHandler = [h copy]; [old release]; }
- (void)applyValue:(float)value {
    if (value == _value) return;
    _value = value;
    void (^changed)(id, float) = _valueChangedHandler;
    if (changed) changed(self, value);
}
- (void)dealloc { [_valueChangedHandler release]; [super dealloc]; }
@end

static QGCButton *make_button(NSString *name, BOOL analog) {
    QGCButton *b = [[QGCButton alloc] init];
    b->_name = [name copy];
    b->_analog = analog;
    return b;
}

static QGCAxis *make_axis(NSString *name) {
    QGCAxis *a = [[QGCAxis alloc] init];
    a->_name = [name copy];
    return a;
}

@implementation QGCDirectionPad
- (QGCButton *)up { return _up; }
- (QGCButton *)down { return _down; }
- (QGCButton *)left { return _left; }
- (QGCButton *)right { return _right; }
- (QGCAxis *)xAxis { return _xAxis; }
- (QGCAxis *)yAxis { return _yAxis; }
- (id)valueChangedHandler { return _valueChangedHandler; }
- (void)setValueChangedHandler:(id)h { id old = _valueChangedHandler; _valueChangedHandler = [h copy]; [old release]; }
- (void)applyX:(float)x y:(float)y {
    BOOL moved = x != _xAxis->_value || y != _yAxis->_value;
    [_xAxis applyValue:x];
    [_yAxis applyValue:y];
    [_up applyValue:y > 0 ? y : 0];
    [_down applyValue:y < 0 ? -y : 0];
    [_left applyValue:x < 0 ? -x : 0];
    [_right applyValue:x > 0 ? x : 0];
    void (^changed)(id, float, float) = _valueChangedHandler;
    if (moved && changed) changed(self, x, y);
}
- (void)dealloc {
    [_up release]; [_down release]; [_left release]; [_right release];
    [_xAxis release]; [_yAxis release]; [_valueChangedHandler release];
    [super dealloc];
}
@end

static QGCDirectionPad *make_dpad(NSString *name) {
    QGCDirectionPad *d = [[QGCDirectionPad alloc] init];
    d->_name = [name copy];
    d->_up = make_button([name stringByAppendingString:@" Up"], NO);
    d->_down = make_button([name stringByAppendingString:@" Down"], NO);
    d->_left = make_button([name stringByAppendingString:@" Left"], NO);
    d->_right = make_button([name stringByAppendingString:@" Right"], NO);
    d->_xAxis = make_axis([name stringByAppendingString:@" X Axis"]);
    d->_yAxis = make_axis([name stringByAppendingString:@" Y Axis"]);
    return d;
}

@implementation QGCGamepad
- (id)init {
    if ((self = [super init])) {
        _buttonA = make_button(@"Button A", YES);
        _buttonB = make_button(@"Button B", YES);
        _buttonX = make_button(@"Button X", YES);
        _buttonY = make_button(@"Button Y", YES);
        _leftShoulder = make_button(@"Left Shoulder", YES);
        _rightShoulder = make_button(@"Right Shoulder", YES);
        _leftTrigger = make_button(@"Left Trigger", YES);
        _rightTrigger = make_button(@"Right Trigger", YES);
        _menu = make_button(@"Button Menu", NO);
        _options = make_button(@"Button Options", NO);
        _home = make_button(@"Button Home", NO);
        _leftStickButton = make_button(@"Left Thumbstick Button", NO);
        _rightStickButton = make_button(@"Right Thumbstick Button", NO);
        _dpad = make_dpad(@"Direction Pad");
        _leftThumbstick = make_dpad(@"Left Thumbstick");
        _rightThumbstick = make_dpad(@"Right Thumbstick");
    }
    return self;
}
- (QGCController *)controller { return _controller; }
- (QGCButton *)buttonA { return _buttonA; }
- (QGCButton *)buttonB { return _buttonB; }
- (QGCButton *)buttonX { return _buttonX; }
- (QGCButton *)buttonY { return _buttonY; }
- (QGCButton *)leftShoulder { return _leftShoulder; }
- (QGCButton *)rightShoulder { return _rightShoulder; }
- (QGCButton *)leftTrigger { return _leftTrigger; }
- (QGCButton *)rightTrigger { return _rightTrigger; }
- (QGCButton *)buttonMenu { return _menu; }
- (QGCButton *)buttonOptions { return _options; }
- (QGCButton *)buttonHome { return _home; }
- (QGCButton *)leftThumbstickButton { return _leftStickButton; }
- (QGCButton *)rightThumbstickButton { return _rightStickButton; }
- (QGCDirectionPad *)dpad { return _dpad; }
- (QGCDirectionPad *)leftThumbstick { return _leftThumbstick; }
- (QGCDirectionPad *)rightThumbstick { return _rightThumbstick; }
- (id)valueChangedHandler { return _valueChangedHandler; }
- (void)setValueChangedHandler:(id)h { id old = _valueChangedHandler; _valueChangedHandler = [h copy]; [old release]; }

/* GCPhysicalInputProfile: the same elements, by name. */
- (NSDictionary *)buttons {
    QGCButton *all[] = {_buttonA, _buttonB, _buttonX, _buttonY, _leftShoulder, _rightShoulder,
        _leftTrigger, _rightTrigger, _menu, _options, _home, _leftStickButton, _rightStickButton};
    NSMutableDictionary *d = [NSMutableDictionary dictionary];
    for (unsigned long i = 0; i < sizeof all / sizeof all[0]; i++) [d setObject:all[i] forKey:all[i]->_name];
    return d;
}
- (NSDictionary *)dpads {
    return [NSDictionary dictionaryWithObjectsAndKeys:_dpad, _dpad->_name,
        _leftThumbstick, _leftThumbstick->_name, _rightThumbstick, _rightThumbstick->_name, nil];
}
- (NSDictionary *)axes { return [NSDictionary dictionary]; }
- (NSSet *)allButtons { return [NSSet setWithArray:[[self buttons] allValues]]; }
- (NSSet *)allDpads { return [NSSet setWithArray:[[self dpads] allValues]]; }
- (NSSet *)allAxes { return [NSSet set]; }
- (NSSet *)allElements { return [[self allButtons] setByAddingObjectsFromSet:[self allDpads]]; }
- (NSDictionary *)elements {
    NSMutableDictionary *d = [NSMutableDictionary dictionaryWithDictionary:[self buttons]];
    [d addEntriesFromDictionary:[self dpads]];
    return d;
}
- (id)objectForKeyedSubscript:(id)key { return [[self elements] objectForKey:key]; }

- (void)applyPad:(const qgp_pad *)p {
    uint32_t b = p->buttons;
#define BTN(bit) ((b >> (bit)) & 1 ? 1.0f : 0.0f)
    [_buttonA applyValue:BTN(QGP_BUTTON_A)];
    [_buttonB applyValue:BTN(QGP_BUTTON_B)];
    [_buttonX applyValue:BTN(QGP_BUTTON_X)];
    [_buttonY applyValue:BTN(QGP_BUTTON_Y)];
    [_leftShoulder applyValue:BTN(QGP_BUTTON_LEFT_SHOULDER)];
    [_rightShoulder applyValue:BTN(QGP_BUTTON_RIGHT_SHOULDER)];
    [_leftTrigger applyValue:p->lt];
    [_rightTrigger applyValue:p->rt];
    [_menu applyValue:BTN(QGP_BUTTON_MENU)];
    [_options applyValue:BTN(QGP_BUTTON_OPTIONS)];
    [_home applyValue:BTN(QGP_BUTTON_HOME)];
    [_leftStickButton applyValue:BTN(QGP_BUTTON_LEFT_STICK)];
    [_rightStickButton applyValue:BTN(QGP_BUTTON_RIGHT_STICK)];
    [_dpad applyX:BTN(QGP_DPAD_RIGHT) - BTN(QGP_DPAD_LEFT) y:BTN(QGP_DPAD_UP) - BTN(QGP_DPAD_DOWN)];
#undef BTN
    [_leftThumbstick applyX:p->lx y:p->ly];
    [_rightThumbstick applyX:p->rx y:p->ry];
    void (^changed)(id, id) = _valueChangedHandler;
    if (changed) changed(self, nil);
}
- (void)dealloc {
    [_buttonA release]; [_buttonB release]; [_buttonX release]; [_buttonY release];
    [_leftShoulder release]; [_rightShoulder release]; [_leftTrigger release]; [_rightTrigger release];
    [_menu release]; [_options release]; [_home release];
    [_leftStickButton release]; [_rightStickButton release];
    [_dpad release]; [_leftThumbstick release]; [_rightThumbstick release];
    [_valueChangedHandler release];
    [super dealloc];
}
@end

@implementation QGCController
- (NSString *)vendorName { return _vendorName; }
- (NSString *)productCategory { return _category; }
- (BOOL)isAttachedToDevice { return YES; }
- (BOOL)isSnapshot { return NO; }
- (NSInteger)playerIndex { return _playerIndex; }
- (void)setPlayerIndex:(NSInteger)index { _playerIndex = index; }
- (QGCGamepad *)extendedGamepad { return _gamepad; }
- (QGCGamepad *)physicalInputProfile { return _gamepad; }
- (id)gamepad { return nil; }
- (id)microGamepad { return nil; }
- (id)motion { return nil; }
- (id)light { return nil; }
- (id)haptics { return nil; }
- (id)battery { return nil; }
- (void *)handlerQueue { return _handlerQueue; }
- (void)setHandlerQueue:(void *)queue {
    void *old = _handlerQueue;
    _handlerQueue = queue ? queue : dispatch_get_main_queue();
    dispatch_retain(_handlerQueue);
    dispatch_release(old);
}
- (id)controllerPausedHandler { return _pausedHandler; }
- (void)setControllerPausedHandler:(id)h { id old = _pausedHandler; _pausedHandler = [h copy]; [old release]; }
- (void)dealloc {
    [_vendorName release]; [_category release]; [_gamepad release]; [_pausedHandler release];
    dispatch_release(_handlerQueue);
    [super dealloc];
}
@end


/* Keys for -[GCPhysicalInputProfile buttons] / dpads, which Darling's stub
 * framework does not define (the client would read nil). Apple's values; the
 * paddle, touchpad and haptic-locality strings are for elements this bridge
 * does not have, so they only keep lookups from failing on a nil key. */
id GCInputButtonA = @"Button A";
id GCInputButtonB = @"Button B";
id GCInputButtonX = @"Button X";
id GCInputButtonY = @"Button Y";
id GCInputButtonMenu = @"Button Menu";
id GCInputButtonOptions = @"Button Options";
id GCInputLeftShoulder = @"Left Shoulder";
id GCInputRightShoulder = @"Right Shoulder";
id GCInputLeftTrigger = @"Left Trigger";
id GCInputRightTrigger = @"Right Trigger";
id GCInputLeftThumbstick = @"Left Thumbstick";
id GCInputRightThumbstick = @"Right Thumbstick";
id GCInputLeftThumbstickButton = @"Left Thumbstick Button";
id GCInputRightThumbstickButton = @"Right Thumbstick Button";
id GCInputDirectionPad = @"Direction Pad";
id GCInputXboxPaddleOne = @"Button Paddle 1";
id GCInputXboxPaddleTwo = @"Button Paddle 2";
id GCInputXboxPaddleThree = @"Button Paddle 3";
id GCInputXboxPaddleFour = @"Button Paddle 4";
id GCInputDualShockTouchpadButton = @"DualShock Touchpad Button";
id GCInputDualShockTouchpadOne = @"DualShock Touchpad One";
id GCInputDualShockTouchpadTwo = @"DualShock Touchpad Two";
id GCHapticsLocalityHandles = @"Handles";
id GCHapticsLocalityLeftHandle = @"LeftHandle";
id GCHapticsLocalityRightHandle = @"RightHandle";
id GCHapticsLocalityTriggers = @"Triggers";
id GCHapticsLocalityLeftTrigger = @"LeftTrigger";
id GCHapticsLocalityRightTrigger = @"RightTrigger";
double GCHapticDurationInfinite = __builtin_inf();

/* ---- state shared between the reader thread and the main queue ---- */

static NSMutableArray *controllers;   /* QGCController, guarded by @synchronized(controllers) */
static QGCController *slot_controller[QGP_MAX_PADS];
static uint32_t previous_buttons[QGP_MAX_PADS];

static id product_category(const qgp_pad *p) {
    if (p->vendor == 0x054c) return @"DualSense";
    if (p->vendor == 0x057e) return @"Switch Pro Controller";
    if (p->vendor == 0x045e || qgp_label_xy(p->vendor, p->name)) return @"Xbox One";
    return @"Generic";
}

static id vendor_name(const qgp_pad *p) {
    return [NSString stringWithUTF8String:p->name];
}

static void controller_connected(int slot, qgp_pad pad) {
    QGCController *c = [[QGCController alloc] init];
    c->_slot = slot;
    c->_vendorName = [vendor_name(&pad) copy];
    c->_category = [product_category(&pad) copy];
    c->_playerIndex = -1; /* GCControllerPlayerIndexUnset */
    c->_handlerQueue = dispatch_get_main_queue();
    dispatch_retain(c->_handlerQueue);
    c->_gamepad = [[QGCGamepad alloc] init];
    c->_gamepad->_controller = c;
    [c->_gamepad applyPad:&pad];
    previous_buttons[slot] = pad.buttons;
    slot_controller[slot] = c;
    @synchronized(controllers) { [controllers addObject:c]; }
    [[NSNotificationCenter defaultCenter] postNotificationName:@"GCControllerDidConnectNotification" object:c];
}

static void controller_changed(int slot, qgp_pad pad) {
    QGCController *c = slot_controller[slot];
    if (!c) return;
    uint32_t pressed_now = pad.buttons & ~previous_buttons[slot];
    previous_buttons[slot] = pad.buttons;
    [c->_gamepad applyPad:&pad];
    void (^paused)(id) = c->_pausedHandler;
    if (paused && (pressed_now & (1u << QGP_BUTTON_MENU))) paused(c);
}

static void controller_disconnected(int slot) {
    QGCController *c = slot_controller[slot];
    if (!c) return;
    slot_controller[slot] = nil;
    @synchronized(controllers) { [controllers removeObject:c]; }
    [[NSNotificationCenter defaultCenter] postNotificationName:@"GCControllerDidDisconnectNotification" object:c];
    [c release];
}

static void *reader_main(void *unused) {
    (void)unused;
    pthread_setname_np("quadra-gamepad");
    for (;;) {
        qgp_event events[2 * QGP_MAX_PADS];
        int n = qgp_wait(1000, events, 2 * QGP_MAX_PADS);
        for (int i = 0; i < n; i++) {
            int slot = events[i].slot;
            const qgp_pad *pad = qgp_pad_get(slot);
            qgp_pad copy;
            if (pad) copy = *pad;
            int type = events[i].type;
            if (type != QGP_EVENT_DISCONNECTED && !pad) continue;
            dispatch_async(dispatch_get_main_queue(), ^{
                if (type == QGP_EVENT_CONNECTED) controller_connected(slot, copy);
                else if (type == QGP_EVENT_CHANGED) controller_changed(slot, copy);
                else controller_disconnected(slot);
            });
        }
    }
    return 0;
}

/* ---- replacing the stub class's methods ---- */

static NSArray *gc_controllers(id self, SEL cmd) {
    @synchronized(controllers) { return [NSArray arrayWithArray:controllers]; }
}
static id gc_current(id self, SEL cmd) {
    @synchronized(controllers) { return [controllers firstObject]; }
}
static void gc_start_discovery(id self, SEL cmd, void (^done)(void)) {
    if (done) dispatch_async(dispatch_get_main_queue(), done);
}
static void gc_stop_discovery(id self, SEL cmd) {}
static BOOL gc_no(id self, SEL cmd) { return NO; }
static BOOL gc_no_arg(id self, SEL cmd, id arg) { return NO; }
static void gc_ignore_bool(id self, SEL cmd, BOOL value) {}

static int installed;

static void replace_class_method(Class meta, SEL sel, IMP imp, const char *types) {
    class_replaceMethod(meta, sel, imp, types);
}

static void try_install(void) {
    if (installed) return;
    Class gc = objc_getClass("GCController");
    if (!gc) return;
    installed = 1;
    controllers = [[NSMutableArray alloc] init];
    Class meta = object_getClass(gc);
    replace_class_method(meta, sel_registerName("controllers"), (IMP)gc_controllers, "@@:");
    replace_class_method(meta, sel_registerName("current"), (IMP)gc_current, "@@:");
    replace_class_method(meta, sel_registerName("startWirelessControllerDiscoveryWithCompletionHandler:"),
                         (IMP)gc_start_discovery, "v@:@?");
    replace_class_method(meta, sel_registerName("stopWirelessControllerDiscovery"), (IMP)gc_stop_discovery, "v@:");
    replace_class_method(meta, sel_registerName("shouldMonitorBackgroundEvents"), (IMP)gc_no, "c@:");
    replace_class_method(meta, sel_registerName("setShouldMonitorBackgroundEvents:"), (IMP)gc_ignore_bool, "v@:c");
    replace_class_method(meta, sel_registerName("supportsHIDDevice:"), (IMP)gc_no_arg, "c@:^{__IOHIDDevice=}");
    void *thread = 0;
    pthread_create(&thread, 0, reader_main, 0);
    pthread_detach(thread);
}

static void image_added(const void *header, long slide) { try_install(); }

__attribute__((constructor)) static void quadra_gamepad_init(void) {
    const char *off = getenv("QUADRA_GAMEPAD");
    if (off && off[0] == '0') return;
    _dyld_register_func_for_add_image(image_added);
    try_install();
}
