// CoreML is missing from Darling, and RobloxPlayer links it (not weakly), so
// without it the client does not load. Its CoreML backend loads a model with
// +[MLModel modelWithContentsOfURL:(configuration:)error:] (after
// [[MLModelConfiguration alloc] init] and setComputeUnits:); no model comes
// back and Roblox logs "Unable to load model".
#import "stub.h"

@interface MLFeatureValue : NSObject @end
@implementation MLFeatureValue
STUB_RESOLVERS
@end
@interface MLModel : NSObject @end
@implementation MLModel
STUB_RESOLVERS
+ (id)modelWithContentsOfURL:(id)url error:(id *)error {
    (void)url;
    if (error)
        *error = 0;
    return 0;
}
+ (id)modelWithContentsOfURL:(id)url configuration:(id)configuration error:(id *)error {
    (void)url;
    (void)configuration;
    if (error)
        *error = 0;
    return 0;
}
@end
@interface MLModelConfiguration : NSObject @end
@implementation MLModelConfiguration
STUB_RESOLVERS
- (void)setComputeUnits:(long)units { (void)units; }
@end
@interface MLMultiArray : NSObject @end
@implementation MLMultiArray
STUB_RESOLVERS
@end
@interface MLPredictionOptions : NSObject @end
@implementation MLPredictionOptions
STUB_RESOLVERS
@end
