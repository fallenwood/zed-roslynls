namespace ZedRoslynLS;

using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;

public partial class ProcessMonitor
{
    private readonly int? parentProcessId;
    private readonly ILspLogger logger;

    public ProcessMonitor(ILspLogger logger)
    {
        this.logger = logger;
        parentProcessId = GetParentProcessId();
    }

    public int? ParentProcessId => parentProcessId;

    public async Task<bool> WaitForParentExit(CancellationTokenSource cancellationTokenSource)
    {
        if (parentProcessId == null)
        {
            this.logger.WriteLineAsync("Invalid parent process ID.").AsTask().Wait();
            return false;
        }

        try
        {
            while (!cancellationTokenSource.IsCancellationRequested)
            {
                var parent = Process.GetProcessById(parentProcessId.Value);

                if (parent.HasExited)
                {
                    this.logger.WriteLineAsync("Detected parent process exit.").AsTask().Wait();
                    return true;
                }

                // this.logger.WriteLineAsync("Detected parent process running.").AsTask().Wait();
                await Task.Delay(TimeSpan.FromSeconds(60), cancellationTokenSource.Token);
            }
        }
        catch (ArgumentException)
        {
            this.logger.WriteLineAsync("Detected parent process exit.").AsTask().Wait();
            // Expected when the parent process has exited.
        }
        catch (Exception e)
        {
            this.logger.WriteLineAsync($"Unexpected error while monitoring parent process. {e}").AsTask().Wait();
        }

        return true;
    }

    private static int? GetParentProcessId()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return GetParentProcessIdWindows();
        }
        else if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
        {
            return GetParentProcessIdLinux();
        }
        return null;
    }

    private static int? GetParentProcessIdWindows()
    {
        try
        {
            using var process = Process.GetCurrentProcess();
            var buffer = new byte[IntPtr.Size * 6];
            int returnLength = 0;

            int status = NtQueryInformationProcess(process.Handle, 0, buffer, buffer.Length, ref returnLength);
            if (status != 0) return null;

            int parentPid = BitConverter.ToInt32(buffer, IntPtr.Size * 5);
            return parentPid;
        }
        catch {}

        return null;
    }

    static int? GetParentProcessIdLinux()
    {
        try
        {
            foreach (var line in File.ReadLines("/proc/self/status"))
            {
                if (line.StartsWith("PPid:"))
                {
                    var parts = line.Split(':');
                    return int.Parse(parts[1].Trim());
                }
            }
        }
        catch { }
        return null;
    }

    [LibraryImport("ntdll.dll")]
    public static partial int NtQueryInformationProcess(IntPtr processHandle, int processInformationClass, byte[] processInformation, int processInformationLength, ref int returnLength);
}
