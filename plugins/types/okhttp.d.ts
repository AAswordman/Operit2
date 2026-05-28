/**
 * Type definitions for OkHttp3 JavaScript API
 * 
 * This file provides TypeScript declarations for the OkHttp wrapper,
 * enabling code completion and type checking when using the API.
 */

/**
 * Configuration options for OkHttpClient
 */
interface OkHttpConfig {
    timeouts: {
        connect: number;
        read: number;
        write: number;
    };
    followRedirects: boolean;
    retryOnConnectionFailure: boolean;
    interceptors: Array<(request: HttpRequest) => HttpRequest | void>;
}

/**
 * HTTP request representation
 */
interface HttpRequest {
    url: string;
    method: string;
    headers: Record<string, string>;
    body?: any;
    bodyType?: 'text' | 'json' | 'form' | 'multipart';
    formParams?: Record<string, string>;
    multipartParams?: Array<{ name: string; value: string; contentType?: string }>;
    execute(options?: OkHttpExecuteOptions): Promise<OkHttpResponse>;
}

interface HttpStreamEvent {
    type: 'response_started' | 'chunk';
    url: string;
    statusCode?: number;
    statusMessage?: string;
    headers?: Record<string, string>;
    contentType?: string;
    chunk?: string;
    chunkIndex?: number;
    receivedBytes?: number;
}

interface OkHttpExecuteOptions {
    onIntermediateResult?: (event: HttpStreamEvent) => void;
}

/**
 * HTTP response representation
 */
interface OkHttpResponse {
    /**
     * The raw response data
     */
    raw: HttpResponseData;

    /**
     * HTTP status code
     */
    statusCode: number;

    /**
     * HTTP status message
     */
    statusMessage: string;

    /**
     * Response headers
     */
    headers: Record<string, string>;

    /**
     * Response content as string
     */
    content: string;

    /**
     * Content-Type header value
     */
    contentType: string;

    /**
     * Size of the response content in bytes
     */
    size: number;

    /**
     * Parse response body as JSON
     */
    json(): any;

    /**
     * Get response body as text
     */
    text(): string;

    /**
     * Get response body as Base64 encoded string
     */
    bodyAsBase64(): string;

    /**
     * Check if response was successful (status 200-299)
     */
    isSuccessful(): boolean;
}

/**
 * OkHttpClientBuilder class for configuring a client
 */
declare class OkHttpClientBuilder {
    /**
     * Set the connection timeout in milliseconds
     */
    connectTimeout(timeout: number): OkHttpClientBuilder;

    /**
     * Set the read timeout in milliseconds
     */
    readTimeout(timeout: number): OkHttpClientBuilder;

    /**
     * Set the write timeout in milliseconds
     */
    writeTimeout(timeout: number): OkHttpClientBuilder;

    /**
     * Configure whether to follow redirects
     */
    followRedirects(follow: boolean): OkHttpClientBuilder;

    /**
     * Configure whether to retry on connection failure
     */
    retryOnConnectionFailure(retry: boolean): OkHttpClientBuilder;

    /**
     * Add a request interceptor
     */
    addInterceptor(interceptor: (request: HttpRequest) => HttpRequest | void): OkHttpClientBuilder;

    /**
     * Build the configured client
     */
    build(): OkHttpClient;
}

/**
 * OkHttpClient class for executing HTTP requests
 */
declare class OkHttpClient {
    /**
     * Create a new request builder
     */
    newRequest(): RequestBuilder;

    /**
     * Execute an HTTP request
     */
    execute(request: HttpRequest, options?: OkHttpExecuteOptions): Promise<OkHttpResponse>;

    /**
     * Shorthand method for GET requests
     */
    get(url: string, headers?: Record<string, string>): Promise<OkHttpResponse>;

    /**
     * Shorthand method for POST requests
     */
    post(url: string, body: any, headers?: Record<string, string>): Promise<OkHttpResponse>;

    /**
     * Shorthand method for PUT requests
     */
    put(url: string, body: any, headers?: Record<string, string>): Promise<OkHttpResponse>;

    /**
     * Shorthand method for DELETE requests
     */
    delete(url: string, headers?: Record<string, string>): Promise<OkHttpResponse>;

    streamExecute(request: HttpRequest, onIntermediateResult: (event: HttpStreamEvent) => void): Promise<OkHttpResponse>;

    /**
     * Create a new client builder
     */
    static newBuilder(): OkHttpClientBuilder;
}

/**
 * RequestBuilder class for building HTTP requests
 */
declare class RequestBuilder {
    /**
     * Set the request URL
     */
    url(url: string): RequestBuilder;

    /**
     * Set the HTTP method
     */
    method(method: string): RequestBuilder;

    /**
     * Set a single header
     */
    header(name: string, value: string): RequestBuilder;

    /**
     * Set multiple headers
     */
    headers(headers: Record<string, string>): RequestBuilder;

    /**
     * Set the request body
     */
    body(body: any, type?: 'text' | 'json' | 'form' | 'multipart'): RequestBuilder;

    /**
     * Set the request body as JSON
     */
    jsonBody(data: any): RequestBuilder;

    /**
     * Add a form parameter
     */
    formParam(name: string, value: string): RequestBuilder;

    /**
     * Add a multipart form parameter
     */
    multipartParam(name: string, value: string, contentType?: string): RequestBuilder;

    /**
     * Build the HTTP request
     */
    build(): HttpRequest;
}

/**
 * Global OkHttp object
 */
declare const OkHttp: {
    /**
     * Create a new client with default configuration
     */
    newClient(): OkHttpClient;

    /**
     * Create a new client builder
     */
    newBuilder(): OkHttpClientBuilder;
}; 
