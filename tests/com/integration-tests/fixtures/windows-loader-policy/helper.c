/* A trusted test DLL with a durable, loader-lock-safe initialization observer.
 * No CRT calls, allocation, locking, threads or module loading in DllMain. */
typedef unsigned long DWORD;
typedef int BOOL;
typedef unsigned short WCHAR;
typedef void *HANDLE;
#define WINAPI __stdcall
#define IMPORT __declspec(dllimport)
IMPORT DWORD WINAPI GetEnvironmentVariableW(const WCHAR *, WCHAR *, DWORD);
IMPORT HANDLE WINAPI CreateFileW(const WCHAR *, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
IMPORT BOOL WINAPI CloseHandle(HANDLE);
IMPORT HANDLE WINAPI GetCurrentProcess(void);
IMPORT BOOL WINAPI TerminateProcess(HANDLE, unsigned int);

/* Static storage avoids a large stack probe while the loader lock is held. */
static WCHAR marker_path[32768];

/* An observer failure must never masquerade as a rejected plugin open. */
static void observer_failed(void) {
    TerminateProcess(GetCurrentProcess(), 98);
    for (;;) { /* TerminateProcess on this process does not return on success. */ }
}

/* Each build supplies a different marker key and origin, with one DLL basename. */
__declspec(dllexport) int nc_policy_helper_origin(void) { return NC_POLICY_ORIGIN; }

/* Only process attachment is observed; failed-load teardown cannot erase evidence. */
BOOL WINAPI DllMain(HANDLE module, DWORD reason, void *reserved) {
    (void)module;
    (void)reserved;
    if (reason == 1) {
        DWORD length = GetEnvironmentVariableW((const WCHAR *)NC_POLICY_MARKER, marker_path, 32768);
        HANDLE file;
        if (length == 0 || length >= 32768) observer_failed();
        file = CreateFileW(marker_path, 0x40000000, 0, 0, 1, 0x80, 0);
        if (file == (HANDLE)(-1)) observer_failed();
        if (!CloseHandle(file)) observer_failed();
    }
    return 1;
}
