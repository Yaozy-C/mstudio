// Headless transport regression: exercise real MLT and SDL with a dummy device.
#include "player.h"
#include "surface.h"
#include <atomic>
#include <chrono>
#include <cstdlib>
#include <iostream>
#include <stdexcept>
#include <thread>
using namespace std::chrono_literals;
static std::atomic<int> presentations{0};
void* surface_create(void*) { return &presentations; }
void surface_rect(void*,double,double,double,double,bool) {}
void surface_present(void*,const uint8_t*,int,int) { ++presentations; }
void surface_destroy(void*) {}
struct Status { int frame, playing, total; uint64_t shown, skipped; };
Status status() {
    Status s;
    studio_player_status(&s.frame,&s.playing,&s.total,&s.shown,&s.skipped);
    return s;
}
void require(bool condition, const char* message) {
    if (!condition) throw std::runtime_error(message);
}
template<class Predicate> void until(Predicate predicate, const char* message) {
    for (int i=0;i<300;i++) {
        if (predicate()) return;
        std::this_thread::sleep_for(10ms);
    }
    auto s=status(); std::cerr << "frame=" << s.frame << " playing=" << s.playing << " shown=" << s.shown << "\n";
    require(false,message);
}
int main(int argc,char** argv) {
    try {
        require(argc==2,"MLT SDK path required");
        const char* xml=R"(<mlt><producer id="p" in="0" out="59">
          <property name="mlt_service">color</property><property name="resource">red</property>
          </producer><playlist id="timeline"><entry producer="p" in="0" out="59"/></playlist></mlt>)";
        for (int pass=0;pass<6;pass++) {
            const char* error=studio_player_open(nullptr,argv[1],xml,320,240,30);
            if (error) throw std::runtime_error(error);
            until([]{return status().shown>0;},"initial frame not presented");
            require(status().total==60,"unexpected duration");
            studio_player_seek(30);
            until([]{return status().frame==30 && status().shown>1;},"paused seek failed");
            studio_player_play(1);
            until([]{return status().frame>=36;},"play after seek failed");
            studio_player_play(0);
            std::this_thread::sleep_for(150ms);
            int paused=status().frame;
            std::this_thread::sleep_for(150ms);
            require(!status().playing && status().frame==paused,"pause did not settle");
            auto shown=status().shown;
            for (int i=0;i<20;i++) studio_player_seek(i*2);
            until([shown]{auto s=status();return s.frame==38 && s.shown>shown;},"rapid seek did not settle on latest frame");
            studio_player_seek(58);
            studio_player_play(1);
            until([]{return status().frame==59 && !status().playing;},"end detection failed");
            studio_player_seek(0);
            studio_player_play(1);
            until([]{auto s=status();return s.frame>0 && s.frame<20 && s.playing;},"replay failed");
            void* retired=studio_player_retire();
            require(retired && status().total==0,"retired player still exposed");
            std::thread worker([retired]{studio_player_stop_retired(retired);});
            worker.join();
            studio_player_destroy_retired(retired);
        }
        std::cout << "Native transport passed: seek, pause, rapid seek, end, replay, retire/reopen\n";
    } catch (const std::exception& e) {
        studio_player_close();
        std::cerr<<e.what()<<'\n'; return 1;
    }
}
