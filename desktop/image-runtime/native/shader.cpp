#include <string>

namespace mlx { namespace core { namespace metal {
void set_metallib_path(const std::string& path);
}}}

extern "C" void mstudio_mlx_shader_path(const char* path) {
    mlx::core::metal::set_metallib_path(std::string(path));
}
