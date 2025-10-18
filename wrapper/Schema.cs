namespace ZedRoslynLS;

using System.Text.Json.Serialization;

public sealed class OpenSolutionNotifiation(string solution)
{
    [JsonPropertyName("jsonrpc")]
    public string JsonRpc { get; } = "2.0";
    [JsonPropertyName("method")]
    public string Method { get; } = "solution/open";
    [JsonPropertyName("params")]
    public SolutionParams Params { get; } = new(solution);
}

public sealed class OpenProjectNotification(string[] projects)
{
    [JsonPropertyName("jsonrpc")]
    public string JsonRpc { get; } = "2.0";

    [JsonPropertyName("method")]
    public string Method { get; } = "project/open";

    [JsonPropertyName("params")]
    public ProjectParams Params { get; } = new(projects);
}

public sealed class SolutionParams(string solution)
{
    [JsonPropertyName("solution")]
    public string Solution { get; } = solution;
}

public sealed class ProjectParams(string[] projects)
{
    [JsonPropertyName("projects")]
    public string[] Projects { get; } = projects;
}

public sealed class TextDocumentDiagnosticRequest
{
    [JsonPropertyName("id")]
    public long Id { get; set; }

    [JsonPropertyName("jsonrpc")]
    public string JsonRpc { get; set; } = "2.0";

    [JsonPropertyName("method")]
    public string Method { get; set; } = "textDocument/diagnostic";

    [JsonPropertyName("params")]
    public TextDocumentDiagnosticParams Params { get; set; } = new();
}

public sealed class TextDocumentDiagnosticParams
{
    [JsonPropertyName("textDocument")]
    public TextDocument TextDocument { get; set; } = new();

    [JsonPropertyName("range")]
    public Range Range { get; set; } = new();

    // If present, diangostics not work.
    // [JsonPropertyName("identifier")]
    // public string? Identifier { get; set; } = null;

    // [JsonPropertyName("previousResultId")]
    // public string? PreviousResultId { get; set; } = null;
}

public sealed class TextDocument
{
    [JsonPropertyName("uri")]
    public string Uri { get; set; } = string.Empty;
}

public sealed class Range
{
    [JsonPropertyName("start")]
    public Position Start { get; set; } = new();

    [JsonPropertyName("end")]
    public Position End { get; set; } = new();
}

public sealed class Position
{
    [JsonPropertyName("line")]
    public int Line { get; set; }

    [JsonPropertyName("character")]
    public int Character { get; set; }
}

[JsonSerializable(typeof(OpenSolutionNotifiation))]
[JsonSerializable(typeof(OpenProjectNotification))]
[JsonSerializable(typeof(TextDocumentDiagnosticRequest))]
public partial class LspJsonSerializerContext : JsonSerializerContext
{
}
