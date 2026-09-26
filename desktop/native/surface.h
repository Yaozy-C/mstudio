#pragma once
#include <stdint.h>
void* surface_create(void* parent);
void surface_rect(void*,double,double,double,double,bool);
void surface_present(void*,const uint8_t*,int,int);
void surface_destroy(void*);
