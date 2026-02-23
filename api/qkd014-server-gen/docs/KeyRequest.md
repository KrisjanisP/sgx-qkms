# KeyRequest

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**number** | **i32** | Number of keys requested (default 1) | [optional] [default to None]
**size** | **i32** | Size of each key in bits (default key_size from Status) | [optional] [default to None]
**additional_slave_sae_ids** | **Vec<String>** | Optional list of additional slave SAE IDs for key multicast | [optional] [default to None]
**extension_mandatory** | [**Vec<std::collections::HashMap<String, serde_json::Value>>**](map.md) | Extensions that the KME must support or return 400 | [optional] [default to None]
**extension_optional** | [**Vec<std::collections::HashMap<String, serde_json::Value>>**](map.md) | Extensions that the KME may ignore | [optional] [default to None]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


