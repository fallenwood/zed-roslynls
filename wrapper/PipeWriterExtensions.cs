namespace ZedRoslynLS;

using System.IO.Pipelines;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

public static class PipeWriterExtensions
{
    private const string ContentLengthHeader = "Content-Length";
    public static async ValueTask<byte[]> WriteLspMessageAsync(this PipeWriter writer, string textMessage, CancellationToken cancellationToken)
    {
        var length = Encoding.UTF8.GetByteCount(textMessage);

        var message = $"{ContentLengthHeader}: {length}\r\n\r\n{textMessage}";
        var bytes = Encoding.UTF8.GetBytes(message);

        await writer.WriteAsync(bytes, cancellationToken);

        return bytes;
    }
}
