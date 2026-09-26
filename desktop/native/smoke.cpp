// Headless engine regression harness. SDL dummy audio retains the native scheduling path.
#include "player.h"
#include "surface.h"
#include <thread>
#include <chrono>
#include <fstream>
#include <sstream>
#include <iostream>
#include <atomic>
static std::atomic<int> frames{0};
void* surface_create(void*){return reinterpret_cast<void*>(1);}
void surface_rect(void*,double,double,double,double,bool){}
void surface_destroy(void*){}
void surface_present(void*,const uint8_t*,int,int){frames++;}
int main(int argc,char**argv){
    if(argc!=3)return 2;
    std::ifstream f(argv[2]);std::stringstream text;text<<f.rdbuf();
    if(auto error=studio_player_open(nullptr,argv[1],text.str().c_str(),1080,1920,30)){std::cerr<<error;return 1;}
    std::this_thread::sleep_for(std::chrono::milliseconds(400));
    studio_player_play(1);
    std::this_thread::sleep_for(std::chrono::milliseconds(600));
    studio_player_play(0);
    int before=0,after=0,playing=0,total=0;uint64_t shown=0,skipped=0;
    studio_player_status(&before,&playing,&total,&shown,&skipped);
    std::this_thread::sleep_for(std::chrono::milliseconds(250));
    studio_player_status(&after,&playing,&total,&shown,&skipped);
    if(playing || before!=after){std::cerr<<"pause moved";return 1;}
    studio_player_seek(90);
    std::this_thread::sleep_for(std::chrono::milliseconds(200));
    studio_player_status(&after,&playing,&total,&shown,&skipped);
    if(playing || after!=90){std::cerr<<"seek mismatch "<<after;return 1;}
    for(int round=0;round<2;round++){
        studio_player_seek(0);studio_player_play(1);
        int position=0,playing=1,total=0,last=0;uint64_t shown=0,skipped=0;
        const auto start=std::chrono::steady_clock::now();
        do{
            std::this_thread::sleep_for(std::chrono::milliseconds(15));
            studio_player_status(&position,&playing,&total,&shown,&skipped);
            if(position>last+1)std::cout<<"jump "<<last<<" -> "<<position<<"\n";
            last=position;
            if(std::chrono::steady_clock::now()-start>std::chrono::seconds(9)){std::cerr<<"timeout";return 1;}
        }while(playing);
        double elapsed=std::chrono::duration<double>(std::chrono::steady_clock::now()-start).count();
        std::cout<<"round "<<round<<" seconds "<<elapsed<<" position "<<position<<" total "<<total<<" shown "<<shown<<" skipped "<<skipped<<"\n";
        if(position!=total-1 || elapsed<4.5 || elapsed>6)return 1;
    }
    studio_player_close();return frames>200?0:1;
}
