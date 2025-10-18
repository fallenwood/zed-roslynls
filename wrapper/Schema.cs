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

[JsonSerializable(typeof(OpenSolutionNotifiation))]
[JsonSerializable(typeof(OpenProjectNotification))]
public partial class LspJsonSerializerContext : JsonSerializerContext
{
}
