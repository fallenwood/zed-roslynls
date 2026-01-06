using ConsoleAppFramework;
using System;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using ZedRoslynLS;

// #if false
Debugger.Launch();
// #endif

await ConsoleApp.RunAsync(args,
    static async (string lsp, string projectRoot, string? logFilePath = null, RpcType wrapperRpcType = RpcType.Stdio, RpcType lspRpcType = RpcType.NamedPipe, CancellationToken cancellationToken = default) =>
    {
        var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);

        if (Environment.OSVersion.Platform == PlatformID.Unix && !string.IsNullOrEmpty(lsp))
        {
            var lspRoot = Directory.GetParent(lsp)!.FullName;
            var psi = new ProcessStartInfo
            {
                FileName = "chmod",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true,
            };

            psi.ArgumentList.Add("+r");
            psi.ArgumentList.Add("-R");
            psi.ArgumentList.Add(lspRoot);

            using var process = Process.Start(psi)!;
            process.WaitForExit();
        }

        var defaultLogFilePath = Path.Join(Path.GetTempPath(), "zed-roslynls", $"roslynls-{Path.GetFileNameWithoutExtension(projectRoot)}-{DateTime.Now:yyyyMMdd-HHmmss-fff}.txt");

        ILspLogger logger = string.IsNullOrEmpty(logFilePath)
            ? new LspFileLogger(defaultLogFilePath)
            : new LspFileLogger(logFilePath);

        var processor = MessageProcessor.Create(projectRoot, wrapperRpcType, lsp, lspRpcType, logger);

        _ = Task.Factory.StartNew(async () =>
        {
            var monitor = new ProcessMonitor(logger);

            logger.WriteLineAsync($"Monitoring parent process ID \"{monitor.ParentProcessId}\"").AsTask().Wait();

            var exited = await monitor.WaitForParentExit(cts);
            if (exited)
            {
                logger.WriteLineAsync("Parent process exited. Shutting down wrapper.").AsTask().Wait();
                cts.Cancel();
            }
        }, TaskCreationOptions.LongRunning);

        await processor.ProcessAsync(cts.Token);
    });

public enum RpcType
{
    Stdio,
    NamedPipe,
}
