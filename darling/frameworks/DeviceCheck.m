// DeviceCheck is missing from Darling; RobloxPlayer links it weakly. For an
// "appleintegrity" challenge it asks [DCDevice currentDevice] and, if that
// device isSupported, for a token; no device means the unsupported path.
#import "stub.h"

@interface DCDevice : NSObject @end
@implementation DCDevice
STUB_RESOLVERS
+ (id)currentDevice { return 0; }
- (BOOL)isSupported { return 0; }
@end
