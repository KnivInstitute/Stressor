#include <ntddk.h>
#include <windef.h>
#include <intrin.h>

#define DEVICE_NAME L"\\Device\\CpuTempDrv"
#define SYMLINK_NAME L"\\??\\CpuTempDrv"
#define IOCTL_GET_CPU_TEMP CTL_CODE(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)

NTSTATUS DriverEntry(PDRIVER_OBJECT DriverObject, PUNICODE_STRING RegistryPath);
VOID UnloadDriver(PDRIVER_OBJECT DriverObject);
NTSTATUS CreateClose(PDEVICE_OBJECT DeviceObject, PIRP Irp);
NTSTATUS IoControl(PDEVICE_OBJECT DeviceObject, PIRP Irp);

ULONG ReadIntelTemperature();

NTSTATUS DriverEntry(PDRIVER_OBJECT DriverObject, PUNICODE_STRING RegistryPath) {
    UNREFERENCED_PARAMETER(RegistryPath);

    UNICODE_STRING devName = RTL_CONSTANT_STRING(DEVICE_NAME);
    UNICODE_STRING symLink = RTL_CONSTANT_STRING(SYMLINK_NAME);
    PDEVICE_OBJECT deviceObject = NULL;

    NTSTATUS status = IoCreateDevice(DriverObject, 0, &devName, FILE_DEVICE_UNKNOWN, 0, FALSE, &deviceObject);
    if (!NT_SUCCESS(status)) return status;

    status = IoCreateSymbolicLink(&symLink, &devName);
    if (!NT_SUCCESS(status)) {
        IoDeleteDevice(deviceObject);
        return status;
    }

    DriverObject->MajorFunction[IRP_MJ_CREATE] = CreateClose;
    DriverObject->MajorFunction[IRP_MJ_CLOSE] = CreateClose;
    DriverObject->MajorFunction[IRP_MJ_DEVICE_CONTROL] = IoControl;
    DriverObject->DriverUnload = UnloadDriver;

    DbgPrintEx(DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL, "CpuTempDrv: Driver loaded\n");

    return STATUS_SUCCESS;
}

VOID UnloadDriver(PDRIVER_OBJECT DriverObject) {
    UNICODE_STRING symLink = RTL_CONSTANT_STRING(SYMLINK_NAME);
    IoDeleteSymbolicLink(&symLink);
    IoDeleteDevice(DriverObject->DeviceObject);

    DbgPrintEx(DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL, "CpuTempDrv: Driver unloaded\n");
}

NTSTATUS CreateClose(PDEVICE_OBJECT DeviceObject, PIRP Irp) {
    UNREFERENCED_PARAMETER(DeviceObject);
    Irp->IoStatus.Status = STATUS_SUCCESS;
    Irp->IoStatus.Information = 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return STATUS_SUCCESS;
}

NTSTATUS IoControl(PDEVICE_OBJECT DeviceObject, PIRP Irp) {
    UNREFERENCED_PARAMETER(DeviceObject);
    PIO_STACK_LOCATION stack = IoGetCurrentIrpStackLocation(Irp);
    NTSTATUS status = STATUS_INVALID_DEVICE_REQUEST;
    ULONG_PTR info = 0;

    if (stack->Parameters.DeviceIoControl.IoControlCode == IOCTL_GET_CPU_TEMP) {
        if (stack->Parameters.DeviceIoControl.OutputBufferLength >= sizeof(ULONG)) {
            ULONG temp = ReadIntelTemperature();
            if (temp == 0xFFFFFFFF) {
                status = STATUS_NOT_SUPPORTED;
            } else {
                *(ULONG*)Irp->AssociatedIrp.SystemBuffer = temp;
                info = sizeof(ULONG);
                status = STATUS_SUCCESS;
            }
        } else {
            status = STATUS_BUFFER_TOO_SMALL;
        }
    }

    Irp->IoStatus.Status = status;
    Irp->IoStatus.Information = info;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return status;
}

ULONG ReadIntelTemperature() {
    int cpuInfo[4] = {0};
    __cpuid(cpuInfo, 0);
    if (cpuInfo[1] != 0x756e6547 || cpuInfo[2] != 0x6C65746E || cpuInfo[3] != 0x49656E69) {
        DbgPrintEx(DPFLTR_IHVDRIVER_ID, DPFLTR_ERROR_LEVEL, "CpuTempDrv: Not an Intel CPU.\n");
        return 0xFFFFFFFF;
    }

    __cpuid(cpuInfo, 6);
    if (!(cpuInfo[0] & 0x1)) {
        DbgPrintEx(DPFLTR_IHVDRIVER_ID, DPFLTR_ERROR_LEVEL, "CpuTempDrv: Thermal monitoring MSR not supported.\n");
        return 0xFFFFFFFF;
    }

    ULONG64 msr = 0;
    __try {
        msr = __readmsr(0x19C);
    } __except (EXCEPTION_EXECUTE_HANDLER) {
        DbgPrintEx(DPFLTR_IHVDRIVER_ID, DPFLTR_ERROR_LEVEL, "CpuTempDrv: Exception reading MSR.\n");
        return 0xFFFFFFFF;
    }

    ULONG temp_readout = (ULONG)((msr >> 16) & 0x7F);
    ULONG tjmax = 100; // Usually 100C, may differ on some CPUs.

    DbgPrintEx(DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL, "CpuTempDrv: MSR 0x19C readout: %lu (TjMax - temp)\n", temp_readout);

    return tjmax - temp_readout;
}
