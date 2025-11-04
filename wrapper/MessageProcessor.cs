namespace ZedRoslynLS;

using System;
using System.Buffers;
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
    private readonly ILspLogger lspLogger;

    private MessageProcessor(string projectRoot, string solution, string[] projects, string lsp, ILspLogger lspLogger)
    {
        this.projectRoot = projectRoot;
        this.solution = solution;
        this.projects = projects;
        this.lsp = lsp;
        this.lspLogger = lspLogger;
    }

    public static MessageProcessor Create(string projectRoot, string lsp, ILspLogger lspLogger)
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

        return new(projectRoot, solutionUri, projectUris, lsp, lspLogger);
    }

    public async Task ProcessAsync(CancellationToken cancellationToken)
    {
        var logPath = Path.Join(Path.GetTempPath(), "zed-roslynls", Path.GetFileNameWithoutExtension(this.projectRoot));

        var process = new System.Diagnostics.Process();

        var now = DateTime.UtcNow.ToString("yyyyMMddHHmmss");
        var pipeName = $"MicrosoftCodeAnalysisLanguageServer-{now}";

        using var pipe = new System.IO.Pipes.NamedPipeServerStream(pipeName, System.IO.Pipes.PipeDirection.InOut, maxNumberOfServerInstances: 1, System.IO.Pipes.PipeTransmissionMode.Byte, System.IO.Pipes.PipeOptions.Asynchronous);

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
        process.StartInfo.ArgumentList.Add("--pipe");
        process.StartInfo.ArgumentList.Add(pipeName);
        process.Start();

        var consoleInput = Console.OpenStandardInput();
        var consoleOutput = Console.OpenStandardOutput();
        var consoleError = Console.OpenStandardError();
        var serverInput = pipe;
        var serverOutput = pipe;
        var serverError = process.StandardError.BaseStream;

        var outputTask = serverOutput.CopyToAsync(consoleOutput, cancellationToken);
        var errorTask = serverError.CopyToAsync(consoleError, cancellationToken);
        var inputTask = this.ProcessInputAsync(consoleInput, serverInput, cancellationToken);

        await Task.WhenAll(outputTask, inputTask, errorTask);
        await process.WaitForExitAsync(cancellationToken);
    }

    private async Task ProcessOutputAsync(Stream consoleOutput, Stream serverOutput, CancellationToken cancellationToken)
    {
        var reader = PipeReader.Create(serverOutput);
        var writer = PipeWriter.Create(consoleOutput);
    }

    private async Task ProcessInputAsync(Stream consoleInput, Stream serverInput, CancellationToken cancellationToken)
    {
        var reader = PipeReader.Create(consoleInput);
        var writer = PipeWriter.Create(serverInput);

        var initialized = false;

        while (!cancellationToken.IsCancellationRequested)
        {
            var result = await reader.ReadAsync(cancellationToken);
            var buffer = result.Buffer;

            while (TryParseMessage(ref buffer, out var messageText))
            {
                // Workaround for text
                if (messageText.Contains("textDocument/diagnostic", StringComparison.Ordinal))
                {
                    messageText = EnrichTextDocumentDiagnosticRequest(messageText);
                }

                _ = await writer.WriteLspMessageAsync(messageText, cancellationToken);

                if (!initialized && messageText.Contains("initialize", StringComparison.Ordinal))
                {
                    initialized = true;

                    var solutionNotification = new OpenSolutionNotifiation(this.solution);
                    await SendNotificationAsync(writer, solutionNotification, LspJsonSerializerContext.Default.OpenSolutionNotifiation, cancellationToken);

                    var projectNotification = new OpenProjectNotification(this.projects);
                    await SendNotificationAsync(writer, projectNotification, LspJsonSerializerContext.Default.OpenProjectNotification, cancellationToken);

                    await writer.FlushAsync(cancellationToken);
                }
            }

            reader.AdvanceTo(buffer.Start, buffer.End);

            if (result.IsCompleted)
            {
                break;
            }
        }
    }

    // Copilot generated
    private static bool TryParseMessage(ref ReadOnlySequence<byte> buffer, out string messageText)
    {
        messageText = string.Empty;

        if (buffer.Length == 0)
        {
            return false;
        }

        // Search for "Content-Length:" pattern in the buffer
        ReadOnlySpan<byte> contentLengthPattern = "Content-Length:"u8;
        SequencePosition? contentLengthPosition = FindPattern(buffer, contentLengthPattern);

        if (contentLengthPosition == null)
        {
            return false;
        }

        // Check if there's text before Content-Length
        var beforeContentLength = buffer.Slice(0, contentLengthPosition.Value);

        if (beforeContentLength.Length > 0)
        {
            // Return the JSON text that appears before Content-Length
            messageText = Encoding.UTF8.GetString(beforeContentLength);
            buffer = buffer.Slice(contentLengthPosition.Value);
            return true;
        }

        // Now parse the Content-Length header starting from the buffer
        var reader = new SequenceReader<byte>(buffer);

        if (!reader.TryReadTo(out ReadOnlySequence<byte> headerLine, (byte)'\r'))
        {
            return false;
        }

        // Skip the \n after \r
        if (!reader.TryRead(out byte newline) || newline != '\n')
        {
            return false;
        }

        var headerText = Encoding.UTF8.GetString(headerLine);

        // Verify this is Content-Length header
        if (!headerText.StartsWith("Content-Length: ", StringComparison.Ordinal))
        {
            return false;
        }

        // Parse the Content-Length value
        if (!int.TryParse(headerText.Substring(16), out var contentLength))
        {
            return false;
        }

        // Expect \r\n separator
        if (!reader.TryRead(out byte cr) || cr != '\r')
        {
            return false;
        }

        if (!reader.TryRead(out byte lf) || lf != '\n')
        {
            return false;
        }

        // Check if we have enough data for the content
        if (reader.Remaining < contentLength)
        {
            return false;
        }

        // Read the content
        var contentSequence = buffer.Slice(reader.Position, contentLength);
        messageText = Encoding.UTF8.GetString(contentSequence);

        // Advance the buffer past the entire message
        var endPosition = buffer.GetPosition(contentLength, reader.Position);
        buffer = buffer.Slice(endPosition);

        return true;
    }

    // Copilot generated
    private static SequencePosition? FindPattern(ReadOnlySequence<byte> buffer, ReadOnlySpan<byte> pattern)
    {
        if (pattern.Length == 0 || buffer.Length < pattern.Length)
        {
            return null;
        }

        var reader = new SequenceReader<byte>(buffer);

        while (!reader.End)
        {
            var position = reader.Position;

            // Try to match the pattern at current position
            bool matches = true;
            var tempReader = reader;

            for (int i = 0; i < pattern.Length; i++)
            {
                if (!tempReader.TryRead(out byte b) || b != pattern[i])
                {
                    matches = false;
                    break;
                }
            }

            if (matches)
            {
                return position;
            }

            // Move to next byte
            reader.Advance(1);
        }

        return null;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private async Task SendNotificationAsync<T>(PipeWriter writer, T notification, JsonTypeInfo<T> typeInfo, CancellationToken cancellationToken)
    {
        var json = JsonSerializer.Serialize(notification, typeInfo);

        var bytes = await writer.WriteLspMessageAsync(json, cancellationToken);

        await this.lspLogger.WriteAsync(bytes);
        await this.lspLogger.FlushAsync();
    }

    private static string EnrichTextDocumentDiagnosticRequest(string messageText)
    {
        var request = JsonSerializer.Deserialize<TextDocumentDiagnosticRequest>(messageText, LspJsonSerializerContext.Default.TextDocumentDiagnosticRequest);

        if (request?.Params?.TextDocument?.Uri == null)
        {
            return messageText;
        }

        var buffer = File.ReadAllLines(new Uri(request.Params.TextDocument.Uri).LocalPath);

        var lines = buffer.Length;
        var characters = lines == 0 ? 0 : buffer[^1].Length;

        request.Params.Range = new();
        request.Params.Range.Start = new() { Line = 0, Character = 0 };
        request.Params.Range.End = new() { Line = lines, Character = characters };

        messageText = JsonSerializer.Serialize(request, LspJsonSerializerContext.Default.TextDocumentDiagnosticRequest);

        return messageText;
    }
}
