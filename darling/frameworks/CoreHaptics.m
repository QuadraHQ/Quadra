// CoreHaptics is missing from Darling; RobloxPlayer links it weakly. Empty
// classes plus the constants it references keep the game-controller haptics
// code from touching null symbols.
#import "stub.h"

NSString *const CHHapticDynamicParameterIDHapticIntensityControl = @"HapticIntensityControl";
NSString *const CHHapticEventParameterIDHapticIntensity = @"HapticIntensity";
NSString *const CHHapticEventTypeHapticContinuous = @"HapticContinuous";

@interface CHHapticDynamicParameter : NSObject @end
@implementation CHHapticDynamicParameter
STUB_RESOLVERS
@end
@interface CHHapticEvent : NSObject @end
@implementation CHHapticEvent
STUB_RESOLVERS
@end
@interface CHHapticEventParameter : NSObject @end
@implementation CHHapticEventParameter
STUB_RESOLVERS
@end
@interface CHHapticPattern : NSObject @end
@implementation CHHapticPattern
STUB_RESOLVERS
@end
