"""Tests for the Event-to-node conversion (#589, #572, ADR-0026) - against
real google.adk.events.event.Event and google.genai.types shapes, not
mocks."""

from google.adk.events.event import Event
from google.genai import types

from mnem_adk._convert import event_to_node


def _text_event(text: str, *, author: str = "user", invocation_id: str = "inv-1") -> Event:
    return Event(
        id="ev-1",
        author=author,
        invocation_id=invocation_id,
        content=types.Content(role="user", parts=[types.Part(text=text)]),
    )


def test_a_plain_text_event_converts() -> None:
    event = _text_event("My favourite colour is teal.", author="user")
    result = event_to_node("myapp", "u1", "s1", event)
    assert result is not None
    node_id, content, provenance = result
    assert node_id == "myapp:u1:s1:ev-1"
    assert content["parts"][0]["text"] == "My favourite colour is teal."
    assert provenance.source == "user"
    assert provenance.agent_step == "inv-1"
    assert provenance.tool_call is None
    assert provenance.observation is None


def test_an_event_with_no_content_is_skipped() -> None:
    event = Event(id="ev-2", author="user")
    assert event_to_node("myapp", "u1", "s1", event) is None


def test_an_event_with_no_parts_is_skipped() -> None:
    event = Event(id="ev-3", author="user", content=types.Content(role="user", parts=[]))
    assert event_to_node("myapp", "u1", "s1", event) is None


def test_a_function_call_event_captures_tool_call_provenance() -> None:
    event = Event(
        id="ev-4",
        author="agent",
        invocation_id="inv-2",
        content=types.Content(
            role="model",
            parts=[
                types.Part(
                    function_call=types.FunctionCall(
                        id="call-1", name="load_memory", args={"query": "favourite colour"}
                    )
                )
            ],
        ),
    )
    _node_id, _content, provenance = event_to_node("myapp", "u1", "s1", event)
    assert provenance.tool_call == 'load_memory({"query": "favourite colour"})'
    assert provenance.observation is None
    assert provenance.source == "agent"


def test_a_function_response_event_captures_observation_provenance() -> None:
    event = Event(
        id="ev-5",
        author="load_memory",
        content=types.Content(
            role="user",
            parts=[
                types.Part(
                    function_response=types.FunctionResponse(
                        id="call-1", name="load_memory", response={"result": "teal"}
                    )
                )
            ],
        ),
    )
    _node_id, _content, provenance = event_to_node("myapp", "u1", "s1", event)
    assert provenance.observation == '{"result": "teal"}'
    assert provenance.tool_call is None


def test_the_full_content_round_trips_not_just_its_text() -> None:
    event = _text_event("hello")
    _node_id, content, _provenance = event_to_node("myapp", "u1", "s1", event)
    restored = types.Content.model_validate(content)
    assert restored.parts[0].text == "hello"
    assert restored.role == "user"


def test_node_id_is_scoped_by_app_user_session_and_honours_event_id() -> None:
    event = _text_event("hi")
    node_id, _content, _provenance = event_to_node("app2", "u2", "s2", event)
    assert node_id == "app2:u2:s2:ev-1"


def test_an_event_with_no_author_leaves_source_unset() -> None:
    event = Event(
        id="ev-6",
        content=types.Content(role="user", parts=[types.Part(text="hi")]),
    )
    _node_id, _content, provenance = event_to_node("myapp", "u1", "s1", event)
    assert provenance.source is None
