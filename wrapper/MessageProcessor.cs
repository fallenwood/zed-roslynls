namespace ZedRoslynLS;

using System;
using System.IO;
using System.IO.Pipelines;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using System.Threading;
using System.Threading.Tasks;


public sealed class MessageProcessor
{
    private readonly string projectRoot;
    private readonly string solution;
    private readonly string[] projects;
    private readonly string lsp;

    private MessageProcessor(string projectRoot, string solution, string[] projects, string lsp)
    {
        this.projectRoot = projectRoot;
        this.solution = solution;
        this.projects = projects;
        this.lsp = lsp;
    }

    public static MessageProcessor Create(string projectRoot, string lsp)
    {
        var solutions = Array.Empty<string>();
        var projects = Array.Empty<string>();

        if (!string.IsNullOrWhiteSpace(projectRoot) && Directory.Exists(projectRoot))
        {
            solutions = Directory.GetFiles(projectRoot, "*.slnx", SearchOption.TopDirectoryOnly);

            if (solutions.Length == 0)
            {
                solutions = Directory.GetFiles(projectRoot, "*.sln", SearchOption.TopDirectoryOnly);
            }

            projects = Directory.GetFiles(projectRoot, "*.csproj", SearchOption.AllDirectories);
        }

        var solution = solutions.FirstOrDefault() ?? string.Empty;
        var solutionUri = string.Empty;

        if (!string.IsNullOrEmpty(solution))
        {
            solutionUri = new Uri(solution).AbsoluteUri;
        }

        var projectUris = projects.Select(p => new Uri(p).AbsoluteUri).ToArray();

        return new(projectRoot, solutionUri, projectUris, lsp);
    }

    public async Task ProcessAsync(CancellationToken cancellationToken)
    {
        var logPath = Path.Join(Path.GetTempPath(), "zed-roslynls", Path.GetFileNameWithoutExtension(this.projectRoot));

        var process = new System.Diagnostics.Process();
        process.StartInfo.FileName = lsp;
        process.StartInfo.UseShellExecute = false;
        process.StartInfo.RedirectStandardInput = true;
        process.StartInfo.RedirectStandardOutput = true;
        process.StartInfo.RedirectStandardError = true;
        process.StartInfo.CreateNoWindow = true;
        process.StartInfo.ArgumentList.Add("--logLevel");
        process.StartInfo.ArgumentList.Add("Information");
        process.StartInfo.ArgumentList.Add("--extensionLogDirectory");
        process.StartInfo.ArgumentList.Add(logPath);
        process.StartInfo.ArgumentList.Add("--stdio");
        process.Start();

        var consoleInput = Console.OpenStandardInput();
        var consoleOutput = Console.OpenStandardOutput();
        var serverInput = process.StandardInput.BaseStream;
        var serverOutput = process.StandardOutput.BaseStream;

        var outputTask = serverOutput.CopyToAsync(consoleOutput, cancellationToken);

        var inputTask = this.ProcessInputAsync(consoleInput, serverInput, cancellationToken);

        await Task.WhenAll(outputTask, inputTask);
        await process.WaitForExitAsync(cancellationToken);
    }

    private async Task ProcessInputAsync(Stream consoleInput, Stream serverInput, CancellationToken cancellationToken)
    {
        var reader = PipeReader.Create(consoleInput);
        var writer = PipeWriter.Create(serverInput);

        var initialized = false;

        // TODO: workaround for
        // { id = 26, jsonrpc = "2.0", method = "textDocument/diagnostic", params = { range = { ["end"] = { character = 0, line = 37 }, start = { character = 0, line = 0 } }, textDocument = { uri = "file:///home/vbox/workspace/cross/Program.cs" } } }
        while (!cancellationToken.IsCancellationRequested)
        {
            var result = await reader.ReadAsync(cancellationToken);
            var buffer = result.Buffer;

            foreach (var segment in buffer)
            {
                await writer.WriteAsync(segment, cancellationToken);

                // Initialized, skip sending solution/project open notifications.
                if (initialized)
                {
                    continue;
                }

                var text = Encoding.UTF8.GetString(segment.Span);

                if (text.Contains("initialize", StringComparison.Ordinal))
                {
                    initialized = true;

                    var solutionNotification = new OpenSolutionNotifiation(this.solution);
                    await SendNotificationAsync(writer, solutionNotification, LspJsonSerializerContext.Default.OpenSolutionNotifiation, cancellationToken);

                    var projectNotification = new OpenProjectNotification(this.projects);

                    await SendNotificationAsync(writer, projectNotification, LspJsonSerializerContext.Default.OpenProjectNotification, cancellationToken);

                    await writer.FlushAsync(cancellationToken);
                }
            }

            reader.AdvanceTo(buffer.End);

            if (result.IsCompleted)
            {
                break;
            }
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static async Task SendNotificationAsync<T>(PipeWriter writer, T notification, JsonTypeInfo<T> typeInfo, CancellationToken cancellationToken)
    {
        var json = JsonSerializer.Serialize(notification, typeInfo);

        var message = $"Content-Length: {json.Length}\r\n\r\n{json}";
        var bytes = Encoding.UTF8.GetBytes(message);
        await writer.WriteAsync(bytes, cancellationToken);
    }
}
