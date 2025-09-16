
#include "../imgui/imgui.h"
#include "../imgui/imgui_internal.h"
#include "cimgui.h"
#include "cimgui_internal.h"

CIMGUI_API void igItemSize(CONST struct ImRect bb, float text_offset_y)
{
    ImGui::ItemSize(bb, text_offset_y);
}

CIMGUI_API bool igItemAdd(CONST struct ImRect bb, ImGuiID id)
{
    return ImGui::ItemAdd(bb, id);
}
