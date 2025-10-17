using ConsoleAppFramework;
using System.Threading;
using ZedRoslynLS;

await ConsoleApp.RunAsync(args,
    static async (string lsp, string projectRoot, CancellationToken cancellationToken) =>
    {
        var processor = MessageProcessor.Create(projectRoot, lsp);

        await processor.ProcessAsync(cancellationToken);
    });
