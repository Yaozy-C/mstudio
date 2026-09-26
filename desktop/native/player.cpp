#include "player.h"
#include "surface.h"
#include <mlt++/Mlt.h>
#include <atomic>
#include <memory>
#include <string>
#include <algorithm>
#include <cstdlib>
#include <stdexcept>

namespace {
struct Player {
    std::unique_ptr<Mlt::Profile> profile;
    std::unique_ptr<Mlt::Producer> producer;
    std::unique_ptr<Mlt::Consumer> consumer;
    void* surface=nullptr;
    std::atomic<int> position{0};
    std::atomic<bool> playing{false}, ended{false};
    std::atomic<uint64_t> shown{0}, skipped{0};
    std::atomic<int> last{-1};
    int total=0;
    ~Player() {
        if (consumer) {consumer->stop(); consumer.reset();}
        if (surface) surface_destroy(surface);
    }
};
std::unique_ptr<Player> player;
std::string error;
bool initialized=false;
void show(mlt_properties,void* opaque,mlt_event_data data) {
    auto* p=static_cast<Player*>(opaque);
    auto frame=mlt_event_data_to_frame(data);
    int position=mlt_frame_get_position(frame);
    if (p->ended) return;
    int w=p->profile->width(),h=p->profile->height();
    auto format=mlt_image_rgba;
    uint8_t* pixels=nullptr;
    if (!mlt_frame_get_image(frame,&pixels,&format,&w,&h,0) && pixels) {
        surface_present(p->surface,pixels,w,h);
        p->shown++;
    }
    if (p->playing && p->last>=0 && position>p->last+1) p->skipped+=position-p->last-1;
    p->last=position;
    p->position=position;
    if (position>=p->total-1) {
        p->ended=true;
        p->playing=false;
        p->producer->set_speed(0);
    }
}
}
extern "C" const char* studio_player_open(void* parent,const char* root,const char* xml,int width,int height,int fps) {
    try {
        studio_player_close();
        if (!initialized) {
            std::string dir=std::string(root)+"/lib/mlt";
#ifdef _WIN32
            _putenv_s("MLT_DATA",(std::string(root)+"/share/mlt").c_str());
#else
            setenv("MLT_DATA",(std::string(root)+"/share/mlt").c_str(),1);
#endif
            Mlt::Factory::init(dir.c_str());
            initialized=true;
        }
        auto p=std::make_unique<Player>();
        p->profile=std::make_unique<Mlt::Profile>();
        double scale=640.0/std::max(width,height);
        p->profile->set_width(std::max(2,static_cast<int>(width*scale/2+0.5)*2));
        p->profile->set_height(std::max(2,static_cast<int>(height*scale/2+0.5)*2));
        p->profile->set_frame_rate(fps,1);
        p->profile->set_sample_aspect(1,1);
        p->profile->set_display_aspect(width,height);
        p->profile->set_progressive(1);
        p->profile->set_colorspace(709);
        p->profile->set_explicit(1);
        p->producer=std::make_unique<Mlt::Producer>(*p->profile,"xml-string",xml);
        if (!p->producer->is_valid()) throw std::runtime_error("MLT 无法打开时间线");
        p->total=p->producer->get_playtime();
        p->producer->set_speed(0);
        p->producer->set("eof","continue");
        p->surface=surface_create(parent);
        if (!p->surface) throw std::runtime_error("无法创建原生预览区域");
        p->consumer=std::make_unique<Mlt::Consumer>(*p->profile,"sdl2_audio");
        if (!p->consumer->is_valid()) throw std::runtime_error("MLT SDL2 音频输出模块缺失");
        p->consumer->set("real_time",1);
        p->consumer->set("buffer",5);
        p->consumer->set("prefill",2);
        p->consumer->set("scrub_audio",0);
        p->consumer->set("rescale","bilinear");
        p->consumer->set("mlt_image_format","rgba");
        p->consumer->set("frequency",48000);
        p->consumer->set("channels",2);
        mlt_events_listen(p->consumer->get_properties(),p.get(),"consumer-frame-show",show);
        p->consumer->connect(*p->producer);
        if (p->consumer->start()) throw std::runtime_error("MLT 播放设备启动失败");
        p->consumer->set("refresh",1);
        player=std::move(p);
        return nullptr;
    } catch (const std::exception& e) {error=e.what();return error.c_str();}
}
extern "C" void studio_player_rect(double x,double y,double w,double h,int visible) {
    if(player) surface_rect(player->surface,x,y,w,h,visible!=0);
}
extern "C" void studio_player_play(int playing) {
    if(!player)return;
    player->consumer->stop();
    int position=player->position;
    if(playing && position>=player->total-1)position=0;
    player->producer->seek(position);
    player->producer->set_speed(playing?1:0);
    player->last=-1;
    player->ended=false;
    player->playing=playing!=0;
    if(playing)player->consumer->start();
}
extern "C" void studio_player_seek(int frame) {
    if(!player)return;
    player->consumer->stop();
    frame=std::clamp(frame,0,std::max(0,player->total-1));
    player->producer->seek(frame);
    player->position=frame;
    player->last=-1;
    player->ended=false;
    player->producer->set_speed(player->playing?1:0);
    player->consumer->start();
    player->consumer->set("refresh",1);
}
extern "C" void studio_player_close() { player.reset(); }
extern "C" void studio_player_status(int* frame,int* playing,int* total,uint64_t* shown,uint64_t* skipped) {
    *frame=player?player->position.load():0;*playing=player&&player->playing;*total=player?player->total:0;
    *shown=player?player->shown.load():0;*skipped=player?player->skipped.load():0;
}
