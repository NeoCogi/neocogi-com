#include <stdint.h>

#ifdef _WIN32
#define EXPORT __declspec(dllexport)
#define IMPORT __declspec(dllimport)
#else
#define EXPORT
#define IMPORT
#endif

#ifdef NC_ORIGIN_DEPENDENCY
EXPORT int nc_origin_anchor(void) { return 7; }
#elif defined(NC_ORIGIN_FORWARD_ONLY)
/* The linker supplies the export forwarding. This code imports no provider. */
int nc_origin_forward_marker(void) { return 7; }
#else
IMPORT int nc_origin_anchor(void);
EXPORT int nc_origin_uses_import(void) { return nc_origin_anchor(); }
#endif

#if defined(NC_ORIGIN_DEPENDENCY) || defined(NC_ORIGIN_LOCAL_GETTER)
/* A deliberately failing getter with the real signature. Its status proves
   discovery reached this image's code before metadata validation. */
struct NcPluginDescriptor;
EXPORT int32_t nc_get_plugin_descriptor(const struct NcPluginDescriptor **out) {
    *out = 0;
    return nc_origin_anchor() == 7 ? -14 : -4;
}
#endif
