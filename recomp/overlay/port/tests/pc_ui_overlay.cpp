#include "../runtime/ios/pc_ui_overlay.h"
#include <cassert>
#include <filesystem>
#include <chrono>
int main() {
    const auto suffix = std::chrono::steady_clock::now().time_since_epoch().count();
    const auto temp = std::filesystem::temp_directory_path() / ("mw-pc-ui-" + std::to_string(suffix));
    std::string selected = "original";
    assert(select_pc_ui_archive(nullptr, 3, selected) && selected == "original");
    assert(select_pc_ui_archive("", 3, selected) && selected == "original");
    assert(!select_pc_ui_archive(temp.string().c_str(), 3, selected));
    FILE* f = std::fopen(temp.string().c_str(), "wb"); assert(f);
    assert(std::fwrite("abc", 1, 3, f) == 3); std::fclose(f);
    assert(!select_pc_ui_archive(temp.string().c_str(), 4, selected) && selected == "original");
    assert(select_pc_ui_archive(temp.string().c_str(), 3, selected) && selected == temp.string());
    std::filesystem::remove(temp);
    std::puts("PC_UI_OVERLAY PASSED missing/wrong-size/exact-size/no-override");
}
