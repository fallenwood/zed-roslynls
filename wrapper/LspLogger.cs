namespace ZedRoslynLS;

using System;
using System.IO;
using System.IO.Pipelines;
using System.Threading.Tasks;

public interface ILspLogger
{
    public ValueTask<FlushResult> WriteAsync(ReadOnlyMemory<byte> buffer);
    public ValueTask<FlushResult> FlushAsync();
}

public sealed class LspNoopLogger : ILspLogger
{
    public ValueTask<FlushResult> FlushAsync()
    {
        return ValueTask.FromResult(new FlushResult(isCanceled: false, isCompleted: true));
    }

    public ValueTask<FlushResult> WriteAsync(ReadOnlyMemory<byte> buffer)
    {
        return ValueTask.FromResult(new FlushResult(isCanceled: false, isCompleted: true));
    }
}

public sealed class LspFileLogger : ILspLogger, IDisposable
{
    private readonly PipeWriter writer;

    public LspFileLogger(string logFilePath)
    {
        var logDirectory = Path.GetDirectoryName(logFilePath);

        if (!string.IsNullOrEmpty(logDirectory) && !Directory.Exists(logDirectory))
        {
            Directory.CreateDirectory(logDirectory);
        }

        this.writer = PipeWriter.Create(new FileStream(logFilePath, FileMode.Append, FileAccess.Write, FileShare.Read));
    }

    public void Dispose()
    {
        this.writer.Complete();
    }

    public ValueTask<FlushResult> FlushAsync()
    {
        return this.writer.FlushAsync();
    }

    public ValueTask<FlushResult> WriteAsync(ReadOnlyMemory<byte> buffer)
    {
        return this.writer.WriteAsync(buffer);
    }
}
