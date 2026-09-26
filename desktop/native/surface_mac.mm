#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>
#include "surface.h"
#include <memory>
#include <mutex>
// At most one presentation is queued. A slow UI cannot accumulate stale frames.
struct Surface {
    NSView* __strong view;
    NSView* __weak web;
    std::mutex mutex;
    CGImageRef latest=nullptr;
    bool queued=false,alive=true;
    ~Surface(){if(latest)CGImageRelease(latest);}
};
using SurfaceHandle=std::shared_ptr<Surface>;
static NSView* findWeb(NSView* view) {
    if ([view isKindOfClass:NSClassFromString(@"WKWebView")]) return view;
    for (NSView* child in view.subviews) { if (NSView* found=findWeb(child)) return found; }
    return nil;
}
void* surface_create(void* parent) {
    auto s=std::make_shared<Surface>();
    NSWindow* window=(__bridge NSWindow*)parent;
    s->web=findWeb(window.contentView);
    s->view=[[NSView alloc] initWithFrame:NSZeroRect];
    s->view.wantsLayer=YES;
    s->view.layer.backgroundColor=[NSColor colorWithSRGBRed:0.976 green:0.969 blue:0.949 alpha:1].CGColor;
    s->view.layer.contentsGravity=kCAGravityResizeAspect;
    s->view.hidden=YES;
    [window.contentView addSubview:s->view];
    return new SurfaceHandle(s);
}
void surface_rect(void* handle,double x,double y,double w,double h,bool visible) {
    auto s=*static_cast<SurfaceHandle*>(handle);
    NSView* parent=s->view.superview;
    NSView* web=s->web ?: parent;
    // WKWebView extends under the macOS titlebar; its DOM viewport starts
    // below the window's safe content layout area.
    NSRect safe=[parent convertRect:s->view.window.contentLayoutRect toView:web];
    CGFloat inset=web.isFlipped?NSMinY(safe):web.bounds.size.height-NSMaxY(safe);
    CGFloat py=web.isFlipped?y+inset:web.bounds.size.height-inset-y-h;
    s->view.frame=[web convertRect:NSMakeRect(x,py,w,h) toView:parent];
    s->view.hidden=!visible || w<=0 || h<=0;
    s->view.layer.contentsScale=s->view.window.backingScaleFactor;
}
void surface_present(void* handle,const uint8_t* bytes,int w,int h) {
    auto s=*static_cast<SurfaceHandle*>(handle);
    CFDataRef data=CFDataCreate(kCFAllocatorDefault,bytes,(CFIndex)w*h*4);
    CGDataProviderRef provider=CGDataProviderCreateWithCFData(data);
    CGColorSpaceRef color=CGColorSpaceCreateDeviceRGB();
    CGImageRef image=CGImageCreate(w,h,8,32,w*4,color,kCGBitmapByteOrderDefault|kCGImageAlphaLast,provider,nullptr,false,kCGRenderingIntentDefault);
    CGColorSpaceRelease(color);CGDataProviderRelease(provider);CFRelease(data);
    std::lock_guard<std::mutex> guard(s->mutex);
    if(s->latest)CGImageRelease(s->latest);
    s->latest=image;
    if(s->queued || !s->alive)return;
    s->queued=true;
    dispatch_async(dispatch_get_main_queue(),^{
        std::lock_guard<std::mutex> lock(s->mutex);
        s->queued=false;
        if(!s->alive)return;
        [CATransaction begin];[CATransaction setDisableActions:YES];
        s->view.layer.contents=(__bridge id)s->latest;
        [CATransaction commit];
    });
}
void surface_destroy(void* handle) {
    auto holder=static_cast<SurfaceHandle*>(handle);auto s=*holder;
    {std::lock_guard<std::mutex> guard(s->mutex);s->alive=false;}
    [s->view removeFromSuperview];delete holder;
}
