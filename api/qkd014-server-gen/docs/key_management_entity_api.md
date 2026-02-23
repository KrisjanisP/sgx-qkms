# key_management_entity_api

All URIs are relative to *https://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
**getKey**](key_management_entity_api.md#getKey) | **POST** /api/v1/keys/{slave_SAE_ID}/enc_keys | Get keys
**getKeySimple**](key_management_entity_api.md#getKeySimple) | **GET** /api/v1/keys/{slave_SAE_ID}/enc_keys | Get keys (simple GET form)
**getKeyWithIds**](key_management_entity_api.md#getKeyWithIds) | **POST** /api/v1/keys/{master_SAE_ID}/dec_keys | Get keys with key IDs
**getKeyWithIdsSimple**](key_management_entity_api.md#getKeyWithIdsSimple) | **GET** /api/v1/keys/{master_SAE_ID}/dec_keys | Get keys with key ID (simple GET form)
**getStatus**](key_management_entity_api.md#getStatus) | **GET** /api/v1/keys/{slave_SAE_ID}/status | Get status of keys available


# **getKey**
> models::KeyContainer getKey(slave_sae_id, optional)
Get keys

Returns Key container data from the KME to the calling master SAE.  Use POST for all cases other than the specified simple GET forms. 

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **slave_sae_id** | **String**| URL-encoded SAE ID of slave SAE | 
 **optional** | **map[string]interface{}** | optional parameters | nil if no parameters

### Optional Parameters
Optional parameters are passed through a map[string]interface{}.

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **slave_sae_id** | **String**| URL-encoded SAE ID of slave SAE | 
 **key_request** | [**KeyRequest**](KeyRequest.md)|  | 

### Return type

[**models::KeyContainer**](KeyContainer.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getKeySimple**
> models::KeyContainer getKeySimple(slave_sae_id, optional)
Get keys (simple GET form)

Returns Key container data from the KME to the calling master SAE.  GET is allowed only when the Key request is empty, or when only \"number\" and/or \"size\" are used (as URI query parameters). Otherwise use POST. 

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **slave_sae_id** | **String**| URL-encoded SAE ID of slave SAE | 
 **optional** | **map[string]interface{}** | optional parameters | nil if no parameters

### Optional Parameters
Optional parameters are passed through a map[string]interface{}.

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **slave_sae_id** | **String**| URL-encoded SAE ID of slave SAE | 
 **number** | **i32**| Number of keys requested (default 1) | 
 **size** | **i32**| Size of each key in bits (default is key_size from Status). Some KMEs require a multiple of 8 and may return 400 with message \"size shall be a multiple of 8\".  | 

### Return type

[**models::KeyContainer**](KeyContainer.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getKeyWithIds**
> models::KeyContainer getKeyWithIds(master_sae_id, key_ids)
Get keys with key IDs

Returns Key container from the KME to the calling slave SAE.  401 is returned if the SAE ID of the requestor was not an SAE ID that called \"Get key\" resulting in the return of any of the requested key IDs. 

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **master_sae_id** | **String**| URL-encoded SAE ID of master SAE | 
  **key_ids** | [**KeyIds**](KeyIds.md)|  | 

### Return type

[**models::KeyContainer**](KeyContainer.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getKeyWithIdsSimple**
> models::KeyContainer getKeyWithIdsSimple(master_sae_id, key_id)
Get keys with key ID (simple GET form)

Returns Key container from the KME to the calling slave SAE.  GET is allowed only when a single key_ID is specified and no extensions are used. Otherwise use POST with Key IDs data format.  401 is returned if the SAE ID of the requestor was not an SAE ID that called \"Get key\" resulting in the return of any of the requested key IDs. 

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **master_sae_id** | **String**| URL-encoded SAE ID of master SAE | 
  **key_id** | [****](.md)| ID of the key (UUID) | 

### Return type

[**models::KeyContainer**](KeyContainer.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getStatus**
> models::Status getStatus(slave_sae_id)
Get status of keys available

Returns Status from a KME to the calling SAE. Status contains information on keys available to be requested by a master SAE for a specified slave SAE. 

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **slave_sae_id** | **String**| URL-encoded SAE ID of slave SAE | 

### Return type

[**models::Status**](Status.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

