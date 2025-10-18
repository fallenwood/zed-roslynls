using ConsoleAppFramework;
using System;
using System.Diagnostics;
using System.IO;
using System.Threading;
using ZedRoslynLS;

await ConsoleApp.RunAsync(args,
    static async (string lsp, string projectRoot, CancellationToken cancellationToken) =>
    {
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

        var processor = MessageProcessor.Create(projectRoot, lsp);

        await processor.ProcessAsync(cancellationToken);
    });
