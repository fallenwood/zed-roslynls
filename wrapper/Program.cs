using ConsoleAppFramework;
using System;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

await ConsoleApp.RunAsync(args,
  static async (string lsp, string projectRoot, CancellationToken cancellationToken) =>
  {
      var processor = MessageProcessor.Create(projectRoot, lsp);

      await processor.ProcessAsync(cancellationToken);
  });

public class MessageProcessor
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

        return new(projectRoot, solutions.FirstOrDefault() ?? string.Empty, projects, lsp);
    }

    public async Task ProcessAsync(CancellationToken cancellationToken)
    {
        var logPath = Path.Join(Path.GetTempPath(), "zed-roslynls", Path.GetFileNameWithoutExtension(this.projectRoot) + DateTime.UnixEpoch.Ticks.ToString());

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

        var input = Console.OpenStandardInput();
        var lsInput = process.StandardInput.BaseStream;

        var errorTask = process.StandardError.BaseStream.CopyToAsync(Console.OpenStandardError(), cancellationToken);
        var outputTask = process.StandardOutput.BaseStream.CopyToAsync(Console.OpenStandardOutput(), cancellationToken);

        var inputTask = this.PassInputAsync(input, lsInput, cancellationToken);

        await Task.WhenAll(outputTask, errorTask, inputTask);
        await process.WaitForExitAsync(cancellationToken);
    }

    private async Task PassInputAsync(System.IO.Stream input, System.IO.Stream output, CancellationToken cancellationToken)
    {
        {
            using var reader = new StreamReader(input);
            using var writer = new StreamWriter(output) { AutoFlush = true };

            while (!cancellationToken.IsCancellationRequested)
            {
                var line = await reader.ReadLineAsync(cancellationToken);
                if (line == null)
                    break;

                await writer.WriteLineAsync(line);

                if (line.Contains("initialize", StringComparison.Ordinal))
                {
                    var solutionNotification = new Schema.OpenSolutionNotifiation(this.solution);
                    var solutionJson = JsonSerializer.Serialize(solutionNotification, Schema.LspJsonSerializerContext.Default.OpenSolutionNotifiation);

                    await writer.WriteLineAsync(solutionJson);

                    foreach (var project in this.projects)
                    {
                        var projectNotification = new Schema.OpenProjectNotification(project);
                        var projectJson = JsonSerializer.Serialize(projectNotification, Schema.LspJsonSerializerContext.Default.OpenProjectNotification);

                        await writer.WriteLineAsync(projectJson);
                    }

                    break;
                }
            }
        }

        await input.CopyToAsync(output, cancellationToken);
    }
}
