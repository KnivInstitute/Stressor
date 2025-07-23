#include <ntddk.h>
#include <intrin.h>

// Intel constants
#define IOCTL_GET_CPU_TEMP        CTL_CODE(FILE_DEVICE_UNKNOWN, 0x800, METHOD_BUFFERED, FILE_READ_DATA|FILE_WRITE_DATA)
#define MSR_IA32_THERM_STATUS     0x19C
#define INTEL_TJMAX_DEFAULT       100

// AMD PCI/SMU constants
#define AMD_SMU_BUS               0
#define AMD_SMU_DEVICE            0x18
#define AMD_SMU_FUNCTION          3
#define AMD_SMU_TEMP_OFFSET       0xA4

// Missing PCI vendor ID for AMD
#define PCI_VENDOR_ID_AMD         0x1022

// Function prototypes
NTSTATUS DriverEntry(PDRIVER_OBJECT DriverObject, PUNICODE_STRING RegistryPath);
VOID UnloadDriver(PDRIVER_OBJECT DriverObject);
NTSTATUS CreateClose(PDEVICE_OBJECT DeviceObject, PIRP Irp);
NTSTATUS IoControl(PDEVICE_OBJECT DeviceObject, PIRP Irp);
BOOLEAN IsIntelCpu(VOID);
BOOLEAN IsAmdCpu(VOID);
NTSTATUS ReadIntelTemp(ULONG* outTemp);
NTSTATUS ReadAmdTemp(ULONG* outTemp);

NTSTATUS DriverEntry(PDRIVER_OBJECT DriverObject, PUNICODE_STRING RegistryPath) {
    UNREFERENCED_PARAMETER(RegistryPath);

    UNICODE_STRING devName = RTL_CONSTANT_STRING(L"\\Device\\CpuTempDrv");
    UNICODE_STRING symLink = RTL_CONSTANT_STRING(L"\\??\\CpuTempDrv");
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

    DbgPrint("CpuTempDrv loaded.\n");

    return STATUS_SUCCESS;
}

VOID UnloadDriver(PDRIVER_OBJECT DriverObject) {
    UNICODE_STRING symLink = RTL_CONSTANT_STRING(L"\\??\\CpuTempDrv");

    IoDeleteSymbolicLink(&symLink);
    IoDeleteDevice(DriverObject->DeviceObject);

    DbgPrint("CpuTempDrv unloaded.\n");
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
    ULONG tempC = 0;

    if (stack->Parameters.DeviceIoControl.IoControlCode == IOCTL_GET_CPU_TEMP) {
        if (stack->Parameters.DeviceIoControl.OutputBufferLength >= sizeof(ULONG)) {
            if (IsIntelCpu()) {
                status = ReadIntelTemp(&tempC);
            } else if (IsAmdCpu()) {
                status = ReadAmdTemp(&tempC);
            } else {
                status = STATUS_NOT_SUPPORTED;
            }

            if (NT_SUCCESS(status)) {
                *(ULONG*)Irp->AssociatedIrp.SystemBuffer = tempC;
                info = sizeof(ULONG);
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

BOOLEAN IsIntelCpu(VOID) {
    int cpuInfo[4];
    __cpuid(cpuInfo, 0);
    return (cpuInfo[1] == 0x756E6547 && cpuInfo[2] == 0x6C65746E && cpuInfo[3] == 0x49656E69);
}

BOOLEAN IsAmdCpu(VOID) {
    int cpuInfo[4];
    __cpuid(cpuInfo, 0);
    // 'Auth' 'enti' 'cAMD' check as int is tricky, better use literals:
    return (cpuInfo[1] == 0x68747541 /*'Auth'*/ &&
            cpuInfo[2] == 0x444D4163 /*'cAMD'*/ &&
            cpuInfo[3] == 0x6974656E /*'enti'*/);
}

NTSTATUS ReadIntelTemp(ULONG* outTemp) {
    int cpuInfo[4];
    __cpuid(cpuInfo, 6);
    if (!(cpuInfo[0] & 1)) return STATUS_NOT_SUPPORTED;

    ULONG64 msr = 0;
    __try {
        msr = __readmsr(MSR_IA32_THERM_STATUS);
    }
    __except (EXCEPTION_EXECUTE_HANDLER) {
        return STATUS_NOT_SUPPORTED;
    }

    ULONG tempRead = (ULONG)((msr >> 16) & 0x7F);
    *outTemp = INTEL_TJMAX_DEFAULT - tempRead;

    return STATUS_SUCCESS;
}

NTSTATUS ReadAmdTemp(ULONG* outTemp) {
    PCI_SLOT_NUMBER slot = {0};
    slot.u.bits.DeviceNumber = AMD_SMU_DEVICE;
    slot.u.bits.FunctionNumber = AMD_SMU_FUNCTION;

    USHORT vendor = 0;
    ULONG bytesRead = HalGetBusDataByOffset(PCIConfiguration, AMD_SMU_BUS, slot.u.AsULONG,
                                            &vendor, 0, sizeof(vendor));
    if (bytesRead != sizeof(vendor) || vendor != PCI_VENDOR_ID_AMD)
        return STATUS_NOT_SUPPORTED;

    ULONG bar0 = 0;
    bytesRead = HalGetBusDataByOffset(PCIConfiguration, AMD_SMU_BUS, slot.u.AsULONG,
                                      &bar0, FIELD_OFFSET(PCI_COMMON_CONFIG, u.type0.BaseAddresses[0]),
                                      sizeof(bar0));
    if (bytesRead != sizeof(bar0) || bar0 == 0)
        return STATUS_NOT_SUPPORTED;

    bar0 &= ~0xF;

    PHYSICAL_ADDRESS physAddr;
    physAddr.QuadPart = bar0;

    PUCHAR base = MmMapIoSpace(physAddr, 0x1000, MmNonCached);
    if (!base) return STATUS_INSUFFICIENT_RESOURCES;

    ULONG rawVal = READ_REGISTER_ULONG((PULONG)(base + AMD_SMU_TEMP_OFFSET));
    MmUnmapIoSpace(base, 0x1000);

    ULONG deg8 = (rawVal >> 21) & 0x7FF;
    *outTemp = deg8 >> 3; // divide by 8 to get degrees C approx.

    return STATUS_SUCCESS;
}
