/**
 * OkHttp3 JavaScript API
 *
 * Provides a user-friendly interface for HTTP operations using OkHttp3 under the hood.
 * This API simplifies making HTTP requests with features like:
 * - Method chaining for building requests
 * - Convenient handling of headers and request bodies
 * - Simplified response parsing
 * - Request/response interceptors
 * - Cookie management
 */

// OkHttp3 Client builder wrapper
class OkHttpClientBuilder {
    constructor() {
        this._config = {
            timeouts: {
                connect: 10000,
                read: 30000,
                write: 30000
            },
            followRedirects: true,
            retryOnConnectionFailure: true,
            interceptors: []
        };
    }

    // Set connection timeout
    connectTimeout(timeout) {
        this._config.timeouts.connect = timeout;
        return this;
    }

    // Set read timeout
    readTimeout(timeout) {
        this._config.timeouts.read = timeout;
        return this;
    }

    // Set write timeout
    writeTimeout(timeout) {
        this._config.timeouts.write = timeout;
        return this;
    }

    // Set whether to follow redirects
    followRedirects(follow) {
        this._config.followRedirects = follow;
        return this;
    }

    // Set whether to retry on connection failure
    retryOnConnectionFailure(retry) {
        this._config.retryOnConnectionFailure = retry;
        return this;
    }

    // Add a request interceptor
    addInterceptor(interceptor) {
        if (typeof interceptor === 'function') {
            this._config.interceptors.push(interceptor);
        }
        return this;
    }

    // Build the client
    build() {
        return new OkHttpClient(this._config);
    }
}

// OkHttp3 Client wrapper
class OkHttpClient {
    constructor(config) {
        this._config = config || {
            timeouts: {
                connect: 10000,
                read: 30000,
                write: 30000
            },
            followRedirects: true,
            retryOnConnectionFailure: true,
            interceptors: []
        };
    }

    // Create a request builder
    newRequest() {
        return new RequestBuilder(this);
    }

    // Execute the request
    async execute(request, options) {
        // Apply interceptors to the request
        let modifiedRequest = request;
        for (const interceptor of this._config.interceptors) {
            modifiedRequest = interceptor(modifiedRequest) || modifiedRequest;
        }

        // Build parameters for the http_request tool
        const params = {
            url: modifiedRequest.url,
            method: modifiedRequest.method,
            headers: JSON.stringify(modifiedRequest.headers),
            body: modifiedRequest.body,
            body_type: modifiedRequest.bodyType || 'text',
            follow_redirects: this._config.followRedirects,
            connect_timeout: Math.max(1, Math.ceil((this._config.timeouts.connect || 10000) / 1000)),
            read_timeout: Math.max(1, Math.ceil((this._config.timeouts.read || 30000) / 1000)),
            write_timeout: Math.max(1, Math.ceil((this._config.timeouts.write || 30000) / 1000))
        };

        if (options && typeof options.onIntermediateResult === 'function') {
            params.stream = true;
        }

        // Execute the request using the underlying http_request tool
        const response = await toolCall({
            name: "http_request",
            params,
            onIntermediateResult:
                options && typeof options.onIntermediateResult === 'function'
                    ? options.onIntermediateResult
                    : undefined
        });

        // Parse response
        return new Response(response);
    }

    // Shorthand methods
    async get(url, headers) {
        return this.newRequest()
            .url(url)
            .method('GET')
            .headers(headers || {})
            .build()
            .execute();
    }

    async post(url, body, headers) {
        return this.newRequest()
            .url(url)
            .method('POST')
            .headers(headers || {})
            .body(body)
            .build()
            .execute();
    }

    async put(url, body, headers) {
        return this.newRequest()
            .url(url)
            .method('PUT')
            .headers(headers || {})
            .body(body)
            .build()
            .execute();
    }

    async delete(url, headers) {
        return this.newRequest()
            .url(url)
            .method('DELETE')
            .headers(headers || {})
            .build()
            .execute();
    }

    async streamExecute(request, onIntermediateResult) {
        return this.execute(request, { onIntermediateResult });
    }

    // Create a new client builder
    static newBuilder() {
        return new OkHttpClientBuilder();
    }
}

// Request builder
class RequestBuilder {
    constructor(client) {
        this._client = client;
        this._request = {
            url: '',
            method: 'GET',
            headers: {},
            body: undefined,
            bodyType: 'text',
            formParams: {},
            multipartParams: []
        };
    }

    // Set request URL
    url(url) {
        this._request.url = url;
        return this;
    }

    // Set request method
    method(method) {
        this._request.method = method.toUpperCase();
        return this;
    }

    // Set request headers
    header(name, value) {
        this._request.headers[name] = value;
        return this;
    }

    // Set multiple headers at once
    headers(headers) {
        this._request.headers = { ...this._request.headers, ...headers };
        return this;
    }

    // Set request body
    body(body, type = 'text') {
        this._request.body = body;
        this._request.bodyType = type;
        return this;
    }

    // Set JSON body
    jsonBody(data) {
        this._request.body = JSON.stringify(data);
        this._request.bodyType = 'json';
        this._request.headers['Content-Type'] = 'application/json';
        return this;
    }

    // Add form parameter
    formParam(name, value) {
        this._request.formParams[name] = value;
        this._request.bodyType = 'form';
        this._request.headers['Content-Type'] = 'application/x-www-form-urlencoded';
        return this;
    }

    // Add multipart form parameter (for file uploads)
    multipartParam(name, value, contentType) {
        this._request.multipartParams.push({ name, value, contentType });
        this._request.bodyType = 'multipart';
        return this;
    }

    // Finalize and build the request
    build() {
        // Handle form parameters
        if (this._request.bodyType === 'form' && Object.keys(this._request.formParams).length > 0) {
            const formData = Object.entries(this._request.formParams)
                .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
                .join('&');
            this._request.body = formData;
        }

        // Handle multipart form data
        if (this._request.bodyType === 'multipart' && this._request.multipartParams.length > 0) {
            // Multipart requests are handled differently - we'll use multipart_request tool
            return {
                execute: async () => {
                    const params = {
                        url: this._request.url,
                        method: this._request.method,
                        headers: JSON.stringify(this._request.headers),
                        fields: JSON.stringify(this._request.multipartParams)
                    };
                    const response = await toolCall("multipart_request", params);
                    return new Response(response);
                }
            };
        }

        // Return a request object with an execute method
        return {
            ...this._request,
            execute: async (options) => {
                return await this._client.execute(this._request, options);
            }
        };
    }
}

// Response wrapper
class Response {
    constructor(rawResponse) {
        this.raw = rawResponse;
        this.statusCode = rawResponse.statusCode;
        this.statusMessage = rawResponse.statusMessage;
        this.content = rawResponse.content;
        this.headers = this._parseHeaders(rawResponse.headers);
        this.base64Content = rawResponse.contentBase64;
        this.contentType = rawResponse.contentType;
        this.size = rawResponse.size;
    }

    // Parse headers string into object
    _parseHeaders(headersString) {
        const headers = {};
        try {
            if (typeof headersString === 'string') {
                const headerLines = headersString.split('\n');
                for (const line of headerLines) {
                    const [name, value] = line.split(':', 2);
                    if (name && value) {
                        headers[name.trim()] = value.trim();
                    }
                }
            } else if (typeof headersString === 'object') {
                // Headers might already be an object
                return headersString;
            }
        } catch (e) {
            console.error('Error parsing headers:', e);
        }
        return headers;
    }

    // Parse response as JSON
    json() {
        try {
            return JSON.parse(this.content);
        } catch (e) {
            throw new Error(`Failed to parse response as JSON: ${e.message}`);
        }
    }

    // Parse response as text
    text() {
        return this.content;
    }

    // Get response body as Base64 encoded string
    bodyAsBase64() {
        return this.base64Content;
    }

    // Check if response is successful (status code 2xx)
    isSuccessful() {
        return this.statusCode >= 200 && this.statusCode < 300;
    }
}

// Export the OkHttp client
var OkHttp = {
    newClient: () => OkHttpClient.newBuilder().build(),
    newBuilder: () => OkHttpClient.newBuilder()
};

__operitExpose('OkHttpClientBuilder', OkHttpClientBuilder);
__operitExpose('OkHttpClient', OkHttpClient);
__operitExpose('RequestBuilder', RequestBuilder);
__operitExpose('OkHttp', OkHttp);
