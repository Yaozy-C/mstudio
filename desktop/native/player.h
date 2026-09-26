#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
const char* studio_player_open(void* parent, const char* root, const char* xml, int width, int height, int fps);
void studio_player_rect(double x,double y,double width,double height,int visible);
void studio_player_play(int playing);
void studio_player_seek(int frame);
void studio_player_close(void);
void studio_player_status(int* frame,int* playing,int* total,uint64_t* shown,uint64_t* skipped);
#ifdef __cplusplus
}
#endif
