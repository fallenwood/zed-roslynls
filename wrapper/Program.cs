using ConsoleAppFramework;
using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using ZedRoslynLS;

await ConsoleApp.RunAsync(args,
    static async (string lsp, string projectRoot, string? logFilePath = null, CancellationToken cancellationToken = default) =>
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

        ILspLogger logger = string.IsNullOrEmpty(logFilePath)
            ? new LspNoopLogger()
            : new LspFileLogger(logFilePath);

        var processor = MessageProcessor.Create(projectRoot, lsp, logger);

        _ = Task.Factory.StartNew(async () =>
        {
            var monitor = new ProcessMonitor();
            var exited = await monitor.WaitForParentExit(cts);
            if (exited)
            {
                cts.Cancel();
            }
        }, TaskCreationOptions.LongRunning);

        await processor.ProcessAsync(cts.Token);
    });
