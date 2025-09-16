
#include <stdio.h>

#if defined _WIN32 || defined __CYGWIN__
#define API __declspec(dllexport)
#ifndef __GNUC__
#define snprintf sprintf_s
#endif
#else
#define API
#endif

#if defined __cplusplus
#define EXTERN extern "C"
#else
#include <stdarg.h>
#include <stdbool.h>
#define EXTERN extern
#endif

#define CIMGUI_API EXTERN API
#define CONST const

CIMGUI_API void          igItemSize(CONST struct ImRect bb, float text_offset_y);
CIMGUI_API bool          igItemAdd(CONST struct ImRect bb, ImGuiID id);
