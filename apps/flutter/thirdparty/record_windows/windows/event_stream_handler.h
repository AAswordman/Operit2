#include <flutter/event_channel.h>
#include <flutter/encodable_value.h>

using namespace flutter;

template <typename T = EncodableValue>
class EventStreamHandler : public StreamHandler<T> {
public:
    /// Shares only a stream's sink lifetime with queued event deliveries.
    class State {
    public:
        /// Sends an event while the Dart stream is listening.
        void Success(const T& data) {
            if (sink_) sink_->Success(data);
        }

    private:
        friend class EventStreamHandler<T>;
        std::unique_ptr<EventSink<T>> sink_;
    };

    EventStreamHandler() = default;

    virtual ~EventStreamHandler() = default;

    /// Exposes weak ownership so callbacks cannot retain a disposed stream.
    std::weak_ptr<State> state() const { return state_; }

protected:
    std::unique_ptr<StreamHandlerError<T>> OnListenInternal(
        const T* arguments, std::unique_ptr<EventSink<T>>&& events) override {
        state_->sink_ = std::move(events);
        return nullptr;
    }

    std::unique_ptr<StreamHandlerError<T>> OnCancelInternal(
        const T* arguments) override {
        state_->sink_.reset();
        return nullptr;
    }

private:
    std::shared_ptr<State> state_ = std::make_shared<State>();
};
