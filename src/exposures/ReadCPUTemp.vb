Imports System
Imports System.IO
Imports System.Runtime.InteropServices
Imports Microsoft.Win32.SafeHandles

Module CpuTempReader

    Const FILE_DEVICE_UNKNOWN As UInteger = &H22
    Const METHOD_BUFFERED As UInteger = 0
    Const FILE_ANY_ACCESS As UInteger = 0

    Function CTL_CODE(devType As UInteger, func As UInteger, method As UInteger, access As UInteger) As UInteger
        Return ((devType << 16) Or (access << 14) Or (func << 2) Or method)
    End Function

    ReadOnly IOCTL_READ_CPU_TEMP As UInteger = CTL_CODE(FILE_DEVICE_UNKNOWN, &H800, METHOD_BUFFERED, FILE_ANY_ACCESS)

    <DllImport("kernel32.dll", SetLastError:=True)>
    Private Function CreateFile(lpFileName As String, dwDesiredAccess As UInteger,
                                dwShareMode As UInteger, lpSecurityAttributes As IntPtr,
                                dwCreationDisposition As UInteger, dwFlagsAndAttributes As UInteger,
                                hTemplateFile As IntPtr) As SafeFileHandle
    End Function

    <DllImport("kernel32.dll", SetLastError:=True)>
    Private Function DeviceIoControl(hDevice As SafeFileHandle, dwIoControlCode As UInteger,
                                     lpInBuffer As IntPtr, nInBufferSize As UInteger,
                                     <Out> ByRef lpOutBuffer As Integer, nOutBufferSize As UInteger,
                                     ByRef lpBytesReturned As UInteger, lpOverlapped As IntPtr) As Boolean
    End Function

    Sub Main()
        Dim hDevice = CreateFile("\\.\CpuTemp", &HC0000000UI, 0, IntPtr.Zero, 3, 0, IntPtr.Zero)
        If hDevice.IsInvalid Then
            Console.WriteLine("Failed to open driver handle.")
            Return
        End If

        Dim temp As Integer = 0
        Dim bytesReturned As UInteger = 0

        If DeviceIoControl(hDevice, IOCTL_READ_CPU_TEMP, IntPtr.Zero, 0, temp, 4, bytesReturned, IntPtr.Zero) Then
            Console.WriteLine("CPU Temperature: " & temp & "°C")
        Else
            Console.WriteLine("DeviceIoControl failed. Error: " & Marshal.GetLastWin32Error())
        End If
    End Sub

End Module
