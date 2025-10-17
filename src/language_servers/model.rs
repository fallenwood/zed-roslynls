// {
//     "count": 1,
//     "value": [
//         {
//             "id": "a9e6b1c9-1749-47d3-bfce-a5a3b75ed21b",
//             "normalizedName": "microsoft.codeanalysis.languageserver.win-x64",
//             "name": "Microsoft.CodeAnalysis.LanguageServer.win-x64",
//             "protocolType": "NuGet",
//             "url": "https://feeds.dev.azure.com/azure-public/3ccf6661-f8ce-4e8a-bb2e-eff943ddd3c7/_apis/Packaging/Feeds/3c18fd2c-cc7c-4cef-8ed7-20227ab3275b/Packages/a9e6b1c9-1749-47d3-bfce-a5a3b75ed21b",
//             "versions": [
//                 {
//                     "id": "edb8c266-7132-46a1-a901-dfc54da814bf",
//                     "normalizedVersion": "5.3.0-1.25510.11",
//                     "version": "5.3.0-1.25510.11",
//                     "isLatest": true,
//                     "isListed": true,
//                     "storageId": "891F38A638398E99132E0BB52533468823959D15A33BA62370B05B871EE5E75300",
//                     "views": [
//                         {
//                             "id": "fe2443e5-2a5b-44ec-b872-e02560294314",
//                             "name": "Local",
//                             "url": null,
//                             "type": "implicit"
//                         }
//                     ],
//                     "publishDate": "2025-10-11T02:38:05.0521588Z"
//                 }
//             ],
//             "_links": {
//                 "self": {
//                     "href": "https://feeds.dev.azure.com/azure-public/3ccf6661-f8ce-4e8a-bb2e-eff943ddd3c7/_apis/Packaging/Feeds/3c18fd2c-cc7c-4cef-8ed7-20227ab3275b/Packages/a9e6b1c9-1749-47d3-bfce-a5a3b75ed21b"
//                 },
//                 "feed": {
//                     "href": "https://feeds.dev.azure.com/azure-public/3ccf6661-f8ce-4e8a-bb2e-eff943ddd3c7/_apis/Packaging/Feeds/3c18fd2c-cc7c-4cef-8ed7-20227ab3275b"
//                 },
//                 "versions": {
//                     "href": "https://feeds.dev.azure.com/azure-public/3ccf6661-f8ce-4e8a-bb2e-eff943ddd3c7/_apis/Packaging/Feeds/3c18fd2c-cc7c-4cef-8ed7-20227ab3275b/Packages/a9e6b1c9-1749-47d3-bfce-a5a3b75ed21b/Versions"
//                 }
//             }
//         }
//     ]
// }

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuGetPackagesResponse {
    pub count: u32,
    pub value: Vec<NuGetPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuGetPackage {
    pub id: String,
    #[serde(rename = "normalizedName")]
    pub normalized_name: String,
    pub name: String,
    #[serde(rename = "protocolType")]
    pub protocol_type: String,
    pub url: String,
    pub versions: Vec<NuGetPackageVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuGetPackageVersion {
    pub id: String,
    #[serde(rename = "normalizedVersion")]
    pub normalized_version: String,
    pub version: String,
    #[serde(rename = "isLatest")]
    pub is_latest: bool,
    #[serde(rename = "isListed")]
    pub is_listed: bool,
    #[serde(rename = "publishDate")]
    pub publish_date: String,
}
