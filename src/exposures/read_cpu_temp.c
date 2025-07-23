#include <windows.h>
#include <stdio.h>

#define IOCTL_GET_CPU_TEMP CTL_CODE(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)

int main() {
    HANDLE hDevice = CreateFileW(
        L"\\\\.\\CpuTempDrv",
        GENERIC_READ | GENERIC_WRITE,
        0,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );

    if (hDevice == INVALID_HANDLE_VALUE) {
        DWORD err = GetLastError();
        if (err == ERROR_ACCESS_DENIED) {
            printf("Failed to open device: Access denied (error 5).\n");
            printf("Try running this program as Administrator.\n");
        } else {
            printf("Failed to open device. Error: %lu\n", err);
        }
        return 1;
    }

    ULONG temp = 0;
    DWORD bytesReturned = 0;

    BOOL success = DeviceIoControl(
        hDevice,
        IOCTL_GET_CPU_TEMP,
        NULL,
        0,
        &temp,
        sizeof(temp),
        &bytesReturned,
        NULL
    );

    if (!success) {
        DWORD err = GetLastError();
        printf("DeviceIoControl failed. Error: %lu\n", err);
        CloseHandle(hDevice);
        return 1;
    }

    printf("CPU Temperature: %lu °C\n", temp);

    CloseHandle(hDevice);
    return 0;
}
