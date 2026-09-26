#ifdef _WIN32
#include "surface.h"
#include <windows.h>
#include <mutex>
#include <vector>
#include <algorithm>
struct Surface { HWND window=nullptr; std::mutex mutex; std::vector<uint8_t> pixels; int width=0,height=0; bool queued=false; };
static constexpr UINT present=WM_APP+72;
static LRESULT CALLBACK proc(HWND hwnd,UINT msg,WPARAM wp,LPARAM lp) {
    auto s=reinterpret_cast<Surface*>(GetWindowLongPtrW(hwnd,GWLP_USERDATA));
    if(msg==WM_NCCREATE){s=static_cast<Surface*>(reinterpret_cast<CREATESTRUCTW*>(lp)->lpCreateParams);SetWindowLongPtrW(hwnd,GWLP_USERDATA,reinterpret_cast<LONG_PTR>(s));}
    if(s && msg==present){std::lock_guard<std::mutex> lock(s->mutex);s->queued=false;InvalidateRect(hwnd,nullptr,FALSE);return 0;}
    if(s && msg==WM_PAINT){
        PAINTSTRUCT ps;HDC dc=BeginPaint(hwnd,&ps);RECT r;GetClientRect(hwnd,&r);FillRect(dc,&r,static_cast<HBRUSH>(GetStockObject(BLACK_BRUSH)));
        std::lock_guard<std::mutex> lock(s->mutex);
        if(!s->pixels.empty()){
            double scale=std::min(double(r.right)/s->width,double(r.bottom)/s->height);int w=int(s->width*scale),h=int(s->height*scale);
            BITMAPINFO info={};info.bmiHeader.biSize=sizeof(BITMAPINFOHEADER);info.bmiHeader.biWidth=s->width;info.bmiHeader.biHeight=-s->height;info.bmiHeader.biPlanes=1;info.bmiHeader.biBitCount=32;info.bmiHeader.biCompression=BI_RGB;
            SetStretchBltMode(dc,HALFTONE);
            StretchDIBits(dc,(r.right-w)/2,(r.bottom-h)/2,w,h,0,0,s->width,s->height,s->pixels.data(),&info,DIB_RGB_COLORS,SRCCOPY);
        }
        EndPaint(hwnd,&ps);return 0;
    }
    return DefWindowProcW(hwnd,msg,wp,lp);
}
void* surface_create(void* parent){
    static bool registered=false;if(!registered){WNDCLASSW c={};c.lpfnWndProc=proc;c.hInstance=GetModuleHandleW(nullptr);c.lpszClassName=L"MstudioMltPreview";RegisterClassW(&c);registered=true;}
    auto s=new Surface();s->window=CreateWindowExW(0,L"MstudioMltPreview",L"",WS_CHILD|WS_CLIPSIBLINGS,0,0,1,1,static_cast<HWND>(parent),nullptr,GetModuleHandleW(nullptr),s);
    if(!s->window){delete s;return nullptr;}return s;
}
void surface_rect(void* h,double x,double y,double w,double height,bool visible){auto s=static_cast<Surface*>(h);SetWindowPos(s->window,HWND_TOP,int(x),int(y),int(w),int(height),SWP_NOACTIVATE);ShowWindow(s->window,visible?SW_SHOWNA:SW_HIDE);}
void surface_present(void* h,const uint8_t* bytes,int w,int height){auto s=static_cast<Surface*>(h);std::lock_guard<std::mutex> lock(s->mutex);s->width=w;s->height=height;s->pixels.resize(size_t(w)*height*4);for(size_t i=0;i<s->pixels.size();i+=4){s->pixels[i]=bytes[i+2];s->pixels[i+1]=bytes[i+1];s->pixels[i+2]=bytes[i];s->pixels[i+3]=255;}if(!s->queued){s->queued=true;PostMessageW(s->window,present,0,0);}}
void surface_destroy(void* h){auto s=static_cast<Surface*>(h);DestroyWindow(s->window);delete s;}
#endif
