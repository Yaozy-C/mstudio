#import <AppKit/AppKit.h>
#import <Metal/Metal.h>
#import <QuartzCore/CAMetalLayer.h>
#include <stdatomic.h>

// AppKit mutations are main-thread-only. Pixels and command encoding run on a
// blocking Rust worker; GPU completions retain the renderer until resources retire.
@interface MstudioMetalView : NSView
@end
@implementation MstudioMetalView
- (NSView *)hitTest:(NSPoint)point { (void)point; return nil; }
- (void)viewDidChangeBackingProperties {
    [super viewDidChangeBackingProperties];
    CAMetalLayer *layer = (CAMetalLayer *)self.layer;
    layer.contentsScale = self.window.backingScaleFactor ?: 1.0;
    layer.drawableSize = [self convertRectToBacking:self.bounds].size;
}
- (void)setFrameSize:(NSSize)size {
    [super setFrameSize:size];
    if (self.layer) [self viewDidChangeBackingProperties];
}
@end

@interface MstudioMetalPreview : NSObject {
@public
    MstudioMetalView *view;
    CAMetalLayer *layer;
    id<MTLDevice> device;
    id<MTLCommandQueue> queue;
    id<MTLRenderPipelineState> pipeline;
    id<MTLTexture> textures[3];
    atomic_bool busy[3];
    atomic_bool visible;
    atomic_bool closed;
    atomic_bool failed;
    atomic_uint_fast64_t generation;
    NSLock *submission;
    unsigned width, height;
}
@end
@implementation MstudioMetalPreview
@end

static const char *shader =
    "#include <metal_stdlib>\nusing namespace metal;\n"
    "struct V { float4 position [[position]]; float2 uv; };\n"
    "vertex V preview_vertex(uint i [[vertex_id]]) {\n"
    " float2 p[3]={float2(-1,-1),float2(3,-1),float2(-1,3)};\n"
    " V v; v.position=float4(p[i],0,1); v.uv=float2((p[i].x+1)*.5,(1-p[i].y)*.5); return v; }\n"
    "fragment float4 preview_pixel(V v [[stage_in]], texture2d<float> image [[texture(0)]]) {\n"
    " constexpr sampler s(coord::normalized,address::clamp_to_edge,filter::linear);\n"
    " return float4(image.sample(s,v.uv).rgb,1); }\n";

void *mstudio_metal_create(void *webview, unsigned width, unsigned height, char *error, size_t capacity) {
    @autoreleasepool {
        NSCAssert(NSThread.isMainThread, @"Metal view creation requires main thread");
        MstudioMetalPreview *r = [MstudioMetalPreview new];
        r->width = width; r->height = height;
        r->device = MTLCreateSystemDefaultDevice();
        NSError *failure = nil;
        id<MTLLibrary> library = [r->device newLibraryWithSource:@(shader) options:nil error:&failure];
        MTLRenderPipelineDescriptor *desc = [MTLRenderPipelineDescriptor new];
        desc.vertexFunction = [library newFunctionWithName:@"preview_vertex"];
        desc.fragmentFunction = [library newFunctionWithName:@"preview_pixel"];
        desc.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm_sRGB;
        if (library) r->pipeline = [r->device newRenderPipelineStateWithDescriptor:desc error:&failure];
        r->queue = [r->device newCommandQueue];
        MTLTextureDescriptor *texture = [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm_sRGB width:width height:height mipmapped:NO];
        texture.storageMode = MTLStorageModeShared;
        texture.usage = MTLTextureUsageShaderRead;
        BOOL allocated = YES;
        for (unsigned i = 0; i < 3; i++) {
            r->textures[i] = [r->device newTextureWithDescriptor:texture];
            allocated &= r->textures[i] != nil;
            atomic_init(&r->busy[i], false);
        }
        if (!r->device || !r->pipeline || !r->queue || !allocated) {
            snprintf(error, capacity, "%s", failure.localizedDescription.UTF8String ?: "Metal preview resources unavailable");
            return NULL;
        }
        atomic_init(&r->visible, false);
        atomic_init(&r->closed, false);
        atomic_init(&r->failed, false);
        atomic_init(&r->generation, 0);
        r->submission = [NSLock new];
        r->layer = [CAMetalLayer layer];
        r->layer.device = r->device;
        r->layer.pixelFormat = MTLPixelFormatBGRA8Unorm_sRGB;
        r->layer.framebufferOnly = YES;
        r->layer.maximumDrawableCount = 3;
        r->layer.allowsNextDrawableTimeout = YES;
        r->layer.displaySyncEnabled = YES;
        CGColorSpaceRef srgb = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
        r->layer.colorspace = srgb;
        CGColorSpaceRelease(srgb);
        r->view = [[MstudioMetalView alloc] initWithFrame:NSMakeRect(0,0,1,1)];
        r->view.wantsLayer = YES;
        r->view.layer = r->layer;
        r->view.hidden = YES;
        [(__bridge NSView *)webview addSubview:r->view positioned:NSWindowAbove relativeTo:nil];
        return (__bridge_retained void *)r;
    }
}

void mstudio_metal_bounds(void *handle, double x, double y, double w, double h, double viewportW, double viewportH, bool show) {
    NSCAssert(NSThread.isMainThread, @"Metal view layout requires main thread");
    MstudioMetalPreview *r = (__bridge MstudioMetalPreview *)handle;
    NSView *parent = r->view.superview;
    BOOL valid = show && !atomic_load(&r->closed) && parent.window.isVisible && !parent.window.isMiniaturized && viewportW > 0 && viewportH > 0 && w > 0 && h > 0;
    if (valid) {
        // WKWebView fills the titlebar on recent macOS, while innerHeight and
        // DOM coordinates exclude its safe-area inset. Use the content viewport.
        NSRect bounds = parent.safeAreaRect;
        double sx = bounds.size.width / viewportW, sy = bounds.size.height / viewportH;
        double top = parent.isFlipped ? y * sy : bounds.size.height - (y + h) * sy;
        r->view.frame = NSMakeRect(bounds.origin.x + x * sx, bounds.origin.y + top, w * sx, h * sy);
        [r->view viewDidChangeBackingProperties];
    }
    r->view.hidden = !valid;
    atomic_store(&r->visible, valid);
}

// 1 = submitted, 0 = hidden/backpressure, -1 = GPU failure. No unbounded queue.
int mstudio_metal_present(void *handle, const unsigned char *pixels, uint64_t generation) {
    @autoreleasepool {
        MstudioMetalPreview *r = (__bridge MstudioMetalPreview *)handle;
        if (atomic_load(&r->failed)) return -1;
        if (atomic_load(&r->closed) || !atomic_load(&r->visible)) return 0;
        unsigned slot = 3;
        for (unsigned i = 0; i < 3; i++) {
            bool expected = false;
            if (atomic_compare_exchange_strong(&r->busy[i], &expected, true)) { slot = i; break; }
        }
        if (slot == 3) return 0;
        id<CAMetalDrawable> drawable = [r->layer nextDrawable];
        if (!drawable || atomic_load(&r->closed) || atomic_load(&r->generation) != generation) { atomic_store(&r->busy[slot], false); return 0; }
        id<MTLTexture> source = r->textures[slot];
        [source replaceRegion:MTLRegionMake2D(0,0,r->width,r->height) mipmapLevel:0 withBytes:pixels bytesPerRow:r->width*4];
        id<MTLCommandBuffer> command = [r->queue commandBuffer];
        MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
        pass.colorAttachments[0].texture = drawable.texture;
        pass.colorAttachments[0].loadAction = MTLLoadActionDontCare;
        pass.colorAttachments[0].storeAction = MTLStoreActionStore;
        id<MTLRenderCommandEncoder> encoder = [command renderCommandEncoderWithDescriptor:pass];
        if (!command || !encoder) { atomic_store(&r->busy[slot], false); return -1; }
        [encoder setRenderPipelineState:r->pipeline];
        [encoder setFragmentTexture:source atIndex:0];
        [encoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
        [encoder endEncoding];
        // A drawable acquisition can wait. Only submission is serialized against
        // controls, so a seek/close never waits for an occluded drawable.
        [r->submission lock];
        if (atomic_load(&r->closed) || atomic_load(&r->generation) != generation) {
            [r->submission unlock];
            atomic_store(&r->busy[slot], false);
            return 0;
        }
        [command presentDrawable:drawable];
        [command addCompletedHandler:^(id<MTLCommandBuffer> completed) {
            if (completed.status == MTLCommandBufferStatusError) atomic_store(&r->failed, true);
            atomic_store(&r->busy[slot], false);
        }];
        [command commit];
        [r->submission unlock];
        return 1;
    }
}
void mstudio_metal_invalidate(void *handle, uint64_t generation) {
    MstudioMetalPreview *r = (__bridge MstudioMetalPreview *)handle;
    [r->submission lock];
    atomic_store(&r->generation, generation);
    [r->submission unlock];
}
void mstudio_metal_detach(void *handle) {
    NSCAssert(NSThread.isMainThread, @"Metal view removal requires main thread");
    MstudioMetalPreview *r = (__bridge MstudioMetalPreview *)handle;
    atomic_store(&r->closed, true);
    atomic_store(&r->visible, false);
    [r->view removeFromSuperview];
}
void mstudio_metal_destroy(void *handle) {
    mstudio_metal_detach(handle);
    (void)CFBridgingRelease(handle);
}
