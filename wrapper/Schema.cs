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

public sealed class OpenProjectNotification(string project)
{
    [JsonPropertyName("jsonrpc")]
    public string JsonRpc { get; } = "2.0";

    [JsonPropertyName("method")]
    public string Method { get; } = "project/open";

    [JsonPropertyName("params")]
    public ProjectParams Params { get; } = new(project);
}

public sealed class SolutionParams(string solution)
{
    [JsonPropertyName("solution")]
    public string Solution { get; } = solution;
}

public sealed class ProjectParams(string project)
{
    [JsonPropertyName("project")]
    public string Project { get; } = project;
}

[JsonSerializable(typeof(OpenSolutionNotifiation))]
[JsonSerializable(typeof(OpenProjectNotification))]
public partial class LspJsonSerializerContext : JsonSerializerContext
{
}
